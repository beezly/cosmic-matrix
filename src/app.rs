use std::collections::HashMap;
use std::sync::Arc;

use cosmic::iced::widget::image::Handle as ImageHandle;
use cosmic::iced::{Alignment, Length, Subscription};
use cosmic::prelude::*;
use cosmic::{executor, widget, Core};
use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
use matrix_sdk::ruma::events::AnySyncTimelineEvent;
use matrix_sdk::ruma::OwnedRoomId;
use matrix_sdk::ruma::OwnedUserId;
use matrix_sdk::Client;

use mime_guess;

use crate::config;
use crate::matrix;
use crate::matrix::verification as matrix_verification;
use crate::message::{
    CrossSigningStatus, LoginSuccess, MatrixClient, Message, TimelineItem, VerificationInfo,
    VerificationPhase, VerificationStateUpdate,
};
use matrix_sdk::media::{MediaFormat, MediaRequestParameters};
use crate::state::rooms::RoomsState;
use crate::state::timeline::TimelineState;
use crate::ui::login::{self, LoginState};
use crate::ui::timeline::TIMELINE_SCROLLABLE_ID;
use crate::ui::{composer, room_header, timeline as timeline_ui};
use crate::ui::verification as verification_ui;
use cosmic::iced::widget::scrollable::{snap_to, RelativeOffset};

enum AppView {
    Loading,
    Login,
    Main,
}

pub struct App {
    core: Core,
    view: AppView,
    login_state: LoginState,
    login_password: String,
    own_user_id: Option<OwnedUserId>,
    rooms_state: RoomsState,
    timeline_state: TimelineState,
    client: Option<Arc<Client>>,
    homeserver: String,
    cross_signing_status: CrossSigningStatus,
    active_verification: Option<VerificationInfo>,
    pending_incoming: Option<(String, String)>, // (flow_id, sender)
    /// Fetched image data keyed by event_id, stored as a pre-built image Handle.
    images: HashMap<String, ImageHandle>,
}

impl cosmic::Application for App {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = config::APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(mut core: Core, _flags: Self::Flags) -> (Self, cosmic::app::Task<Self::Message>) {
        core.window.content_container = false;

        let has_session = config::load_session().is_some();

        let app = App {
            core,
            view: if has_session {
                AppView::Loading
            } else {
                AppView::Login
            },
            login_state: LoginState::default(),
            login_password: String::new(),
            own_user_id: None,
            rooms_state: RoomsState::default(),
            timeline_state: TimelineState::default(),
            client: None,
            homeserver: String::new(),
            cross_signing_status: CrossSigningStatus::Unknown,
            active_verification: None,
            pending_incoming: None,
            images: HashMap::new(),
        };

        let task = if has_session {
            cosmic::task::future(async {
                match try_restore_session().await {
                    Ok(msg) => msg,
                    Err(e) => {
                        tracing::warn!("Session restore failed: {e}");
                        Message::None
                    }
                }
            })
        } else {
            Task::none()
        };

        (app, task)
    }

    fn update(&mut self, message: Self::Message) -> cosmic::app::Task<Self::Message> {
        match message {
            Message::None => {
                if matches!(self.view, AppView::Loading) {
                    self.view = AppView::Login;
                }
            }

            // -- Login --
            Message::HomeserverChanged(val) => self.login_state.homeserver = val,
            Message::UsernameChanged(val) => self.login_state.username = val,
            Message::PasswordChanged(val) => self.login_state.password = val,
            Message::TogglePasswordVisibility => {
                self.login_state.password_visible = !self.login_state.password_visible;
            }
            Message::LoginSubmit => {
                if self.login_state.loading {
                    return Task::none();
                }
                self.login_state.loading = true;
                self.login_state.error = None;

                let homeserver = self.login_state.homeserver.clone();
                let username = self.login_state.username.clone();
                let password = self.login_state.password.clone();

                return cosmic::task::future(async move {
                    match do_login(&homeserver, &username, &password).await {
                        Ok((client, success)) => {
                            Message::LoginResult(Ok((MatrixClient(client), success)))
                        }
                        Err(e) => Message::LoginResult(Err(e)),
                    }
                });
            }
            Message::LoginResult(result) => {
                self.login_state.loading = false;
                match result {
                    Ok((matrix_client, success)) => {
                        tracing::info!("Logged in as {}", success.user_id);
                        self.homeserver = self.login_state.homeserver.clone();
                        // Save password for UIA bootstrap, then clear from login form
                        self.login_password = self.login_state.password.clone();
                        self.login_state.password.clear();
                        self.own_user_id = Some(success.user_id.clone());
                        self.client = Some(Arc::new(matrix_client.0));
                        self.view = AppView::Main;

                        let client = Arc::clone(self.client.as_ref().unwrap());
                        let uid = success.user_id.to_string();
                        let pw = Some(self.login_password.clone());
                        return cosmic::task::future(async move {
                            matrix_verification::bootstrap_cross_signing(
                                (*client).clone(),
                                uid,
                                pw,
                            )
                            .await
                        });
                    }
                    Err(e) => {
                        self.login_state.error = Some(e);
                    }
                }
            }
            Message::SessionRestored(matrix_client) => {
                tracing::info!("Session restored");
                self.client = Some(Arc::new(matrix_client.0));
                self.view = AppView::Main;
                // Retrieve own user ID from client
                self.own_user_id = self
                    .client
                    .as_ref()
                    .and_then(|c| c.user_id().map(|u| u.to_owned()));

                let client = Arc::clone(self.client.as_ref().unwrap());
                let uid = self
                    .own_user_id
                    .as_ref()
                    .map(|u| u.to_string())
                    .unwrap_or_default();
                return cosmic::task::future(async move {
                    // No password for restored sessions; try without auth
                    matrix_verification::bootstrap_cross_signing((*client).clone(), uid, None)
                        .await
                });
            }

            Message::Logout => {
                self.login_password.clear();
                self.active_verification = None;
                self.pending_incoming = None;
                self.own_user_id = None;
                self.cross_signing_status = CrossSigningStatus::Unknown;
                config::clear_session();
                self.client = None;
                self.rooms_state = RoomsState::default();
                self.timeline_state = TimelineState::default();
                self.login_state = LoginState::default();
                self.images.clear();
                self.view = AppView::Login;
            }

            // -- Sync --
            Message::SyncStarted => {
                tracing::info!("Sync started");
            }
            Message::RoomsUpdated(rooms) => {
                tracing::debug!("Got {} rooms", rooms.len());
                self.rooms_state.update_rooms(rooms);
            }
            Message::SyncError(e) => {
                tracing::error!("Sync error: {e}");
            }

            // -- Room selection --
            Message::SelectRoom(room_id) => {
                if self.rooms_state.selected.as_ref() == Some(&room_id) {
                    return Task::none();
                }
                self.rooms_state.selected = Some(room_id.clone());
                self.timeline_state.clear();
                self.timeline_state.loading = true;
                self.timeline_state.room_id = Some(room_id.clone());

                if let Some(ref client) = self.client {
                    let client = client.clone();
                    return cosmic::task::future(async move {
                        load_timeline_for_room(&client, &room_id).await
                    });
                }
            }
            Message::RoomFilterChanged(val) => {
                self.rooms_state.filter = val;
            }

            // -- Timeline --
            Message::TimelineUpdated(room_id, items, token) => {
                if self.timeline_state.room_id.as_ref() == Some(&room_id) {
                    self.timeline_state.set_timeline(room_id, items, token);
                    let mut tasks: Vec<cosmic::app::Task<Message>> = vec![snap_to(
                        TIMELINE_SCROLLABLE_ID.clone(),
                        RelativeOffset::END,
                    )];
                    if let Some(ref client) = self.client {
                        tasks.extend(spawn_image_fetches(
                            &self.timeline_state.items,
                            &self.images,
                            client,
                        ));
                    }
                    return Task::batch(tasks);
                }
            }
            Message::IncomingEvents(room_id, new_items) => {
                if self.timeline_state.room_id.as_ref() == Some(&room_id) {
                    if !self.timeline_state.at_bottom
                        && !self.timeline_state.unread_marker_inserted
                        && !new_items.is_empty()
                    {
                        self.timeline_state.items.push(TimelineItem::UnreadMarker);
                        self.timeline_state.unread_marker_inserted = true;
                    }
                    let mut image_tasks: Vec<cosmic::app::Task<Message>> = Vec::new();
                    if let Some(ref client) = self.client {
                        image_tasks =
                            spawn_image_fetches(&new_items, &self.images, client);
                    }
                    self.timeline_state.items.extend(new_items);
                    matrix::timeline::apply_continuation_markers(
                        &mut self.timeline_state.items,
                    );
                    if self.timeline_state.at_bottom {
                        let mut tasks: Vec<cosmic::app::Task<Message>> = vec![snap_to(
                            TIMELINE_SCROLLABLE_ID.clone(),
                            RelativeOffset::END,
                        )];
                        tasks.extend(image_tasks);
                        return Task::batch(tasks);
                    } else if !image_tasks.is_empty() {
                        return Task::batch(image_tasks);
                    }
                }
            }
            Message::TimelineScrolled(offset) => {
                self.timeline_state.at_bottom = offset.y >= 0.99;
            }
            Message::ScrollToBottom => {
                return snap_to(
                    TIMELINE_SCROLLABLE_ID.clone(),
                    RelativeOffset::END,
                );
            }
            Message::ComposerChanged(val) => {
                self.timeline_state.composer = val;
            }
            Message::ReplyTo(ctx) => {
                self.timeline_state.reply_to = Some(ctx);
            }
            Message::CancelReply => {
                self.timeline_state.reply_to = None;
            }
            Message::SendMessage => {
                let text = self.timeline_state.composer.trim().to_string();
                if text.is_empty() {
                    return Task::none();
                }
                let room_id = match self.timeline_state.room_id.clone() {
                    Some(id) => id,
                    None => return Task::none(),
                };
                let client = match self.client.clone() {
                    Some(c) => c,
                    None => return Task::none(),
                };

                let reply_event_id = self.timeline_state.reply_to.as_ref()
                    .map(|ctx| ctx.event_id.clone());
                self.timeline_state.reply_to = None;
                self.timeline_state.composer.clear();
                self.timeline_state.sending = true;

                return cosmic::task::future(async move {
                    send_message(&client, &room_id, &text, reply_event_id).await
                });
            }
            Message::MessageSent(_room_id) => {
                self.timeline_state.sending = false;
                return snap_to(
                    TIMELINE_SCROLLABLE_ID.clone(),
                    RelativeOffset::END,
                );
            }
            Message::SendError(e) => {
                self.timeline_state.sending = false;
                tracing::error!("Send failed: {e}");
            }
            // -- Attachments --
            Message::PickAttachment => {
                let room_id = match self.timeline_state.room_id.clone() {
                    Some(id) => id,
                    None => return Task::none(),
                };
                let client = match self.client.clone() {
                    Some(c) => c,
                    None => return Task::none(),
                };
                self.timeline_state.attachment_sending = true;
                return cosmic::task::future(async move {
                    pick_and_send_attachment(&client, &room_id).await
                });
            }
            Message::AttachmentSent(_room_id) => {
                self.timeline_state.attachment_sending = false;
            }
            Message::AttachmentError(e) => {
                self.timeline_state.attachment_sending = false;
                tracing::error!("Attachment failed: {e}");
            }

            // -- Inline images --
            Message::ImageFetched { event_id, data } => {
                self.images.insert(event_id, ImageHandle::from_bytes(data));
            }
            Message::ImageFetchFailed { event_id } => {
                tracing::warn!("Failed to fetch image for event {event_id}");
            }

            Message::LoadMoreHistory => {
                let token = match self.timeline_state.pagination_token.clone() {
                    Some(t) => t,
                    None => return Task::none(),
                };
                let room_id = match self.timeline_state.room_id.clone() {
                    Some(id) => id,
                    None => return Task::none(),
                };
                let client = match self.client.clone() {
                    Some(c) => c,
                    None => return Task::none(),
                };

                self.timeline_state.loading = true;

                return cosmic::task::future(async move {
                    load_more_history(&client, &room_id, &token).await
                });
            }
            Message::HistoryLoaded(room_id, items, token) => {
                if self.timeline_state.room_id.as_ref() == Some(&room_id) {
                    let image_tasks = if let Some(ref client) = self.client {
                        spawn_image_fetches(&items, &self.images, client)
                    } else {
                        Vec::new()
                    };
                    self.timeline_state.prepend_items(items, token);
                    matrix::timeline::dedup_adjacent_date_separators(
                        &mut self.timeline_state.items,
                    );
                    matrix::timeline::apply_continuation_markers(
                        &mut self.timeline_state.items,
                    );
                    if !image_tasks.is_empty() {
                        return Task::batch(image_tasks);
                    }
                }
            }

            // -- Cross-signing --
            Message::BootstrapCrossSigning => {
                if let Some(ref client) = self.client {
                    let client = Arc::clone(client);
                    let uid = self
                        .own_user_id
                        .as_ref()
                        .map(|u| u.to_string())
                        .unwrap_or_default();
                    let pw = Some(self.login_password.clone());
                    return cosmic::task::future(async move {
                        matrix_verification::bootstrap_cross_signing(
                            (*client).clone(),
                            uid,
                            pw,
                        )
                        .await
                    });
                }
            }
            Message::CrossSigningBootstrapped => {
                tracing::info!("Cross-signing bootstrapped");
                if let Some(ref client) = self.client {
                    let client = Arc::clone(client);
                    return cosmic::task::future(async move {
                        matrix_verification::fetch_cross_signing_status((*client).clone()).await
                    });
                }
            }
            Message::CrossSigningBootstrapFailed(e) => {
                tracing::warn!("Cross-signing bootstrap failed: {e}");
            }
            Message::CrossSigningStatusFetched(status) => {
                self.cross_signing_status = status;
            }

            // -- Outgoing self-verification --
            Message::StartVerification => {
                if let (Some(ref client), Some(ref uid)) = (&self.client, &self.own_user_id) {
                    let client = Arc::clone(client);
                    let uid = uid.clone();
                    return cosmic::task::future(async move {
                        matrix_verification::start_self_verification((*client).clone(), uid).await
                    });
                }
            }
            Message::VerificationRequestCreated(flow_id) => {
                let other_id = self
                    .own_user_id
                    .as_ref()
                    .map(|u| u.to_string())
                    .unwrap_or_default();
                self.active_verification = Some(VerificationInfo {
                    flow_id,
                    other_user_id: other_id,
                    phase: VerificationPhase::WaitingForAccept,
                });
                self.pending_incoming = None;
            }

            // -- Incoming verification --
            Message::IncomingVerificationRequest { flow_id, sender } => {
                if self.active_verification.is_none() {
                    self.pending_incoming = Some((flow_id, sender));
                }
            }
            Message::AcceptVerification => {
                if let Some((flow_id, sender)) = self.pending_incoming.take() {
                    if let (Some(ref client), Some(ref _uid)) = (&self.client, &self.own_user_id) {
                        let client = Arc::clone(client);
                        if let Ok(sender_uid) = sender.parse::<OwnedUserId>() {
                            return cosmic::task::future(async move {
                                matrix_verification::accept_incoming_verification(
                                    (*client).clone(),
                                    sender_uid,
                                    flow_id,
                                )
                                .await
                            });
                        }
                    }
                }
            }
            Message::IgnoreVerification => {
                self.pending_incoming = None;
            }

            // -- Subscription-driven state --
            Message::VerificationStateChanged(update) => {
                if let Some(ref mut info) = self.active_verification {
                    match update {
                        VerificationStateUpdate::Accepted => {
                            info.phase = VerificationPhase::SasStarted;
                        }
                        VerificationStateUpdate::EmojiReady(e) => {
                            info.phase = VerificationPhase::ShowingEmoji(e);
                        }
                        VerificationStateUpdate::Done => {
                            info.phase = VerificationPhase::Done;
                        }
                        VerificationStateUpdate::Cancelled(r) => {
                            info.phase = VerificationPhase::Cancelled(r);
                        }
                    }
                }
            }

            // -- User actions on emoji panel --
            Message::VerificationConfirm => {
                if let (Some(ref info), Some(ref client), Some(ref uid)) =
                    (&self.active_verification, &self.client, &self.own_user_id)
                {
                    let client = Arc::clone(client);
                    let uid = uid.clone();
                    let fid = info.flow_id.clone();
                    if let Some(ref mut i) = self.active_verification {
                        i.phase = VerificationPhase::Confirming;
                    }
                    return cosmic::task::future(async move {
                        matrix_verification::confirm_verification((*client).clone(), uid, fid)
                            .await
                    });
                }
            }
            Message::VerificationMismatch => {
                if let (Some(ref info), Some(ref client), Some(ref uid)) =
                    (&self.active_verification, &self.client, &self.own_user_id)
                {
                    let client = Arc::clone(client);
                    let uid = uid.clone();
                    let fid = info.flow_id.clone();
                    return cosmic::task::future(async move {
                        matrix_verification::mismatch_verification((*client).clone(), uid, fid)
                            .await
                    });
                }
            }
            Message::CancelVerification => {
                if let (Some(ref info), Some(ref client), Some(ref uid)) =
                    (&self.active_verification, &self.client, &self.own_user_id)
                {
                    let client = Arc::clone(client);
                    let uid = uid.clone();
                    let fid = info.flow_id.clone();
                    self.active_verification = None;
                    return cosmic::task::future(async move {
                        matrix_verification::cancel_verification((*client).clone(), uid, fid).await
                    });
                }
                self.active_verification = None;
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let sync_sub = if let Some(ref client) = self.client {
            matrix::sync::sync_subscription(client.clone())
        } else {
            Subscription::none()
        };

        let verify_sub = if let (Some(ref client), Some(ref info), Some(ref uid)) =
            (&self.client, &self.active_verification, &self.own_user_id)
        {
            matrix_verification::verification_subscription(
                client.clone(),
                uid.clone(),
                info.flow_id.clone(),
            )
        } else {
            Subscription::none()
        };

        Subscription::batch([sync_sub, verify_sub])
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let content: Element<_> = match self.view {
            AppView::Loading => widget::container(widget::text::body("Loading..."))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .into(),
            AppView::Login => login::login_view(&self.login_state),
            AppView::Main => self.main_view(),
        };
        widget::container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .class(cosmic::theme::Container::Background)
            .into()
    }

    fn header_center(&self) -> Vec<Element<'_, Self::Message>> {
        match self.view {
            AppView::Main => {
                let title = self
                    .rooms_state
                    .selected_room_name()
                    .unwrap_or("Cosmic Matrix");
                vec![widget::text::heading(title).into()]
            }
            _ => vec![widget::text::heading("Cosmic Matrix").into()],
        }
    }

    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        match self.view {
            AppView::Main => {
                let icon_label = match self.cross_signing_status {
                    CrossSigningStatus::Verified => "🔒",
                    CrossSigningStatus::Unverified => "🔓",
                    CrossSigningStatus::Unknown => "?",
                };
                vec![
                    widget::text::body(icon_label).into(),
                    widget::button::text("Verify")
                        .on_press(Message::StartVerification)
                        .into(),
                    widget::button::text("Logout")
                        .on_press(Message::Logout)
                        .into(),
                ]
            }
            _ => vec![],
        }
    }
}

impl App {
    fn main_view(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();

        // Sidebar: room list
        let mut sidebar_col = widget::column()
            .spacing(spacing.space_xxs)
            .width(Length::Fixed(280.0));

        // Room search
        sidebar_col = sidebar_col.push(
            widget::text_input::search_input("Search rooms...", &self.rooms_state.filter)
                .on_input(Message::RoomFilterChanged)
                .on_clear(Message::RoomFilterChanged(String::new())),
        );

        // Room list
        let filtered = self.rooms_state.filtered_rooms();
        if filtered.is_empty() {
            sidebar_col = sidebar_col.push(
                widget::container(widget::text::body("No rooms"))
                    .width(Length::Fill)
                    .align_x(Alignment::Center)
                    .padding(spacing.space_m),
            );
        } else {
            let mut room_list = widget::column().spacing(2);
            for room in &filtered {
                let is_selected = self
                    .rooms_state
                    .selected
                    .as_ref()
                    .is_some_and(|s| s == &room.room_id);

                let mut row = widget::row()
                    .spacing(spacing.space_xs)
                    .align_y(Alignment::Center);

                // Avatar letter
                row = row.push(
                    widget::container(widget::text::heading(
                        room.avatar_letter.to_string(),
                    ))
                    .width(Length::Fixed(32.0))
                    .height(Length::Fixed(32.0))
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center),
                );

                // Room name + last message preview
                let mut info_col = widget::column().spacing(2);
                info_col = info_col.push(widget::text::body(room.name.clone()));
                if let Some(ref preview) = room.last_message {
                    let truncated = if preview.len() > 60 {
                        format!("{}…", &preview[..60])
                    } else {
                        preview.clone()
                    };
                    info_col = info_col.push(widget::text::caption(truncated));
                } else if room.is_encrypted {
                    info_col = info_col.push(widget::text::caption("Encrypted"));
                }
                row = row.push(info_col);

                row = row.push(widget::horizontal_space());

                if room.unread_count > 0 {
                    row = row.push(
                        widget::container(widget::text::caption(
                            room.unread_count.to_string(),
                        ))
                        .padding([2, 6]),
                    );
                }

                let room_id = room.room_id.clone();
                let btn = if is_selected {
                    widget::button::custom(row)
                        .on_press(Message::SelectRoom(room_id))
                        .width(Length::Fill)
                        .class(cosmic::theme::Button::Standard)
                } else {
                    widget::button::custom(row)
                        .on_press(Message::SelectRoom(room_id))
                        .width(Length::Fill)
                        .class(cosmic::theme::Button::Text)
                };

                room_list = room_list.push(btn);
            }
            sidebar_col =
                sidebar_col.push(widget::scrollable(room_list).height(Length::Fill));
        }

        let sidebar = widget::container(sidebar_col)
            .padding(spacing.space_xs)
            .height(Length::Fill);

        // Content area
        let mut content_col = widget::column().width(Length::Fill).height(Length::Fill);

        // Incoming verification banner
        if let Some((_, ref sender)) = self.pending_incoming {
            content_col = content_col
                .push(verification_ui::incoming_verification_banner(sender))
                .push(widget::divider::horizontal::default());
        }

        // Main content: verification panel or room timeline
        if let Some(ref info) = self.active_verification {
            content_col = content_col.push(verification_ui::verification_panel(info));
        } else if self.timeline_state.room_id.is_some() {
            content_col = content_col.push(self.content_view());
        } else {
            content_col = content_col.push(
                widget::container(widget::text::body("Select a room from the sidebar"))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center),
            );
        }

        widget::row()
            .push(sidebar)
            .push(widget::divider::vertical::default())
            .push(content_col)
            .height(Length::Fill)
            .into()
    }

    fn content_view(&self) -> Element<'_, Message> {
        // Room header
        let room_name = self
            .rooms_state
            .selected_room_name()
            .unwrap_or("Unknown Room");

        let selected_room = self.rooms_state.selected.as_ref().and_then(|sel| {
            self.rooms_state
                .rooms
                .iter()
                .find(|r| &r.room_id == sel)
        });
        let is_encrypted = selected_room.map(|r| r.is_encrypted).unwrap_or(false);
        let topic = selected_room.and_then(|r| r.topic.as_deref());

        let header = room_header::room_header_view(room_name, is_encrypted, topic);

        // Timeline
        let timeline = timeline_ui::timeline_view(&self.timeline_state, &self.images);

        // Composer
        let composer = composer::composer_view(&self.timeline_state);

        widget::column()
            .push(header)
            .push(widget::divider::horizontal::default())
            .push(timeline)
            .push(widget::divider::horizontal::default())
            .push(composer)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

// -- Async helpers --

async fn do_login(
    homeserver: &str,
    username: &str,
    password: &str,
) -> Result<(Client, LoginSuccess), String> {
    let client = matrix::client::create_client(homeserver).await?;
    let response = matrix::client::login(&client, username, password).await?;

    matrix::client::save_session_from_client(&client, homeserver)?;

    Ok((
        client,
        LoginSuccess {
            user_id: response.user_id,
            device_id: response.device_id.to_string(),
        },
    ))
}

async fn try_restore_session() -> Result<Message, String> {
    let stored = config::load_session().ok_or("No session")?;
    let client = matrix::client::restore_session(&stored).await?;
    tracing::info!("Session restored for {}", stored.user_id);
    Ok(Message::SessionRestored(MatrixClient(client)))
}

async fn load_timeline_for_room(client: &Client, room_id: &OwnedRoomId) -> Message {
    let room = match client.get_room(room_id) {
        Some(r) => r,
        None => return Message::TimelineUpdated(room_id.clone(), Vec::new(), None),
    };

    match matrix::timeline::load_room_timeline(&room).await {
        Ok((items, token)) => Message::TimelineUpdated(room_id.clone(), items, token),
        Err(e) => {
            tracing::error!("Failed to load timeline: {e}");
            Message::TimelineUpdated(room_id.clone(), Vec::new(), None)
        }
    }
}

async fn send_message(
    client: &Client,
    room_id: &OwnedRoomId,
    text: &str,
    reply_to: Option<String>,
) -> Message {
    let room = match client.get_room(room_id) {
        Some(r) => r,
        None => return Message::SendError("Room not found".to_string()),
    };

    let mut content = RoomMessageEventContent::text_plain(text);
    if let Some(event_id_str) = reply_to {
        use matrix_sdk::ruma::events::relation::InReplyTo;
        use matrix_sdk::ruma::events::room::message::Relation;
        use matrix_sdk::ruma::OwnedEventId;
        if let Ok(eid) = OwnedEventId::try_from(event_id_str.as_str()) {
            content.relates_to = Some(Relation::Reply {
                in_reply_to: InReplyTo::new(eid),
            });
        }
    }
    match room.send(content).await {
        Ok(_) => Message::MessageSent(room_id.clone()),
        Err(e) => Message::SendError(format!("Failed to send: {e}")),
    }
}

async fn pick_and_send_attachment(client: &Client, room_id: &OwnedRoomId) -> Message {
    use cosmic::dialog::file_chooser;
    use matrix_sdk::attachment::AttachmentConfig;

    // Open the native COSMIC file picker
    let response = match file_chooser::open::Dialog::new()
        .title("Choose a file to send")
        .open_file()
        .await
    {
        Ok(r) => r,
        Err(file_chooser::Error::Cancelled) => return Message::None,
        Err(e) => return Message::AttachmentError(e.to_string()),
    };

    let path = match response.url().to_file_path() {
        Ok(p) => p,
        Err(_) => return Message::AttachmentError("Could not resolve file path".into()),
    };

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();

    let mime = mime_guess::from_path(&path).first_or_octet_stream();

    let data = match tokio::fs::read(&path).await {
        Ok(d) => d,
        Err(e) => return Message::AttachmentError(format!("Failed to read file: {e}")),
    };

    let room = match client.get_room(room_id) {
        Some(r) => r,
        None => return Message::AttachmentError("Room not found".into()),
    };

    match room
        .send_attachment(&filename, &mime, data, AttachmentConfig::new())
        .await
    {
        Ok(_) => Message::AttachmentSent(room_id.clone()),
        Err(e) => Message::AttachmentError(format!("Failed to send: {e}")),
    }
}

/// Collect image fetch tasks for any image messages not yet in the cache.
fn spawn_image_fetches(
    items: &[TimelineItem],
    images: &HashMap<String, ImageHandle>,
    client: &Arc<Client>,
) -> Vec<cosmic::app::Task<Message>> {
    let mut tasks = Vec::new();
    for item in items {
        if let TimelineItem::Message(msg) = item {
            if let Some(ref img) = msg.image {
                if !msg.event_id.is_empty() && !images.contains_key(&msg.event_id) {
                    let client = client.clone();
                    let event_id = msg.event_id.clone();
                    let source = img.source.clone();
                    tasks.push(cosmic::task::future(async move {
                        fetch_image_data(client, event_id, source).await
                    }));
                }
            }
        }
    }
    tasks
}

async fn fetch_image_data(
    client: Arc<Client>,
    event_id: String,
    source: matrix_sdk::ruma::events::room::MediaSource,
) -> Message {
    let request = MediaRequestParameters {
        source,
        format: MediaFormat::File,
    };
    match client.media().get_media_content(&request, true).await {
        Ok(data) => Message::ImageFetched { event_id, data },
        Err(e) => {
            tracing::warn!("Image fetch failed for {event_id}: {e}");
            Message::ImageFetchFailed { event_id }
        }
    }
}

async fn load_more_history(
    client: &Client,
    room_id: &OwnedRoomId,
    token: &str,
) -> Message {
    let room = match client.get_room(room_id) {
        Some(r) => r,
        None => return Message::HistoryLoaded(room_id.clone(), Vec::new(), None),
    };

    let display_names = matrix::timeline::build_display_names(&room).await;
    let options = matrix_sdk::room::MessagesOptions::backward().from(Some(token));
    match room.messages(options).await {
        Ok(messages) => {
            let mut items = Vec::new();
            let mut last_date: Option<chrono::NaiveDate> = None;
            for event in messages.chunk.iter().rev() {
                if let Ok(ev) = event.raw().deserialize() {
                    let ts_millis: i64 = ev.origin_server_ts().0.into();
                    let item_date = matrix::timeline::ts_to_naive_date(ts_millis);

                    if let Some(date) = item_date {
                        if last_date.as_ref() != Some(&date) {
                            items.push(crate::message::TimelineItem::DateSeparator(
                                matrix::timeline::format_date_label(date),
                            ));
                            last_date = Some(date);
                        }
                    }

                    match ev {
                        AnySyncTimelineEvent::MessageLike(msg_ev) => {
                            if let Some(item) =
                                matrix::timeline::convert_message_event(&msg_ev, &display_names)
                            {
                                items.push(item);
                            }
                        }
                        _ => {}
                    }
                }
            }
            matrix::timeline::apply_continuation_markers(&mut items);
            Message::HistoryLoaded(room_id.clone(), items, messages.end)
        }
        Err(e) => {
            tracing::error!("Failed to load history: {e}");
            Message::HistoryLoaded(room_id.clone(), Vec::new(), None)
        }
    }
}
