use std::collections::HashMap;
use std::sync::LazyLock;

use cosmic::iced::widget::image::Handle as ImageHandle;
use cosmic::iced::{Alignment, ContentFit, Length};
use cosmic::prelude::*;
use cosmic::widget;
use cosmic::widget::Id;

use crate::message::{Message, ReplyContext, TimelineItem, TimelineMessage};
use crate::state::timeline::TimelineState;
use crate::ui::colors;

pub static TIMELINE_SCROLLABLE_ID: LazyLock<Id> =
    LazyLock::new(|| Id::new("timeline"));

pub fn timeline_view<'a>(
    state: &'a TimelineState,
    images: &'a HashMap<String, ImageHandle>,
) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();

    let mut col = widget::column().spacing(spacing.space_xxs);

    if state.loading {
        col = col.push(
            widget::container(widget::text::body("Loading messages..."))
                .width(Length::Fill)
                .align_x(Alignment::Center)
                .padding(spacing.space_s),
        );
    } else if state.pagination_token.is_some() {
        col = col.push(
            widget::container(
                widget::button::text("Load earlier messages")
                    .on_press(Message::LoadMoreHistory),
            )
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .padding(spacing.space_xxs),
        );
    }

    if state.items.is_empty() && !state.loading {
        col = col.push(
            widget::container(widget::text::body("No messages yet"))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
        );
    } else {
        for item in &state.items {
            col = col.push(render_timeline_item(item, images));
        }
    }

    widget::scrollable(col)
        .id(TIMELINE_SCROLLABLE_ID.clone())
        .on_scroll(|vp| Message::TimelineScrolled(vp.relative_offset()))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn render_timeline_item<'a>(
    item: &'a TimelineItem,
    images: &'a HashMap<String, ImageHandle>,
) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();

    match item {
        TimelineItem::Message(msg) => render_message(msg, images),
        TimelineItem::DateSeparator(date) => {
            widget::container(
                widget::row()
                    .push(widget::divider::horizontal::default())
                    .push(
                        widget::text::caption(date.clone())
                            .width(Length::Shrink),
                    )
                    .push(widget::divider::horizontal::default())
                    .spacing(spacing.space_xs)
                    .align_y(Alignment::Center),
            )
            .padding([spacing.space_xs, 0])
            .width(Length::Fill)
            .into()
        }
        TimelineItem::StateEvent(desc) => widget::container(
            widget::text::caption(desc.clone())
                .width(Length::Fill),
        )
        .padding([spacing.space_xxs, spacing.space_s])
        .width(Length::Fill)
        .align_x(Alignment::Center)
        .into(),
        TimelineItem::UnreadMarker => {
            widget::container(
                widget::row()
                    .push(widget::divider::horizontal::default())
                    .push(
                        widget::text::body("New messages")
                            .width(Length::Shrink),
                    )
                    .push(widget::divider::horizontal::default())
                    .spacing(spacing.space_xs)
                    .align_y(Alignment::Center),
            )
            .padding([spacing.space_xs, 0])
            .width(Length::Fill)
            .into()
        }
    }
}

fn render_message<'a>(
    msg: &'a TimelineMessage,
    images: &'a HashMap<String, ImageHandle>,
) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();

    let mut col = widget::column().spacing(2);

    // Reply quote block (always shown, even for continuations)
    if let (Some(ref sender_id), Some(ref preview)) =
        (&msg.reply_to_sender, &msg.reply_to_body)
    {
        let reply_sender_display = sender_id
            .strip_prefix('@')
            .and_then(|s| s.split(':').next())
            .unwrap_or(sender_id.as_str());
        let reply_col = colors::sender_color(sender_id);
        let quote_block = widget::container(
            widget::row()
                .push(widget::divider::vertical::default())
                .push(
                    widget::column()
                        .push(
                            widget::text::caption(reply_sender_display)
                                .class(reply_col),
                        )
                        .push(widget::text::caption(preview.as_str()))
                        .spacing(1),
                )
                .spacing(spacing.space_xs),
        )
        .padding([spacing.space_xxs, spacing.space_xs])
        .width(Length::Fill);
        col = col.push(quote_block);
    }

    // Show sender header for non-continuations, or always when message is a reply
    if !msg.is_continuation || msg.reply_to_sender.is_some() {
        let sender_col = colors::sender_color(&msg.sender);
        let mut header = widget::row().spacing(spacing.space_xs);
        if msg.is_emote {
            header = header.push(
                widget::text::heading(format!("* {}", msg.sender_display))
                    .class(sender_col),
            );
        } else {
            header = header.push(
                widget::text::heading(msg.sender_display.clone())
                    .class(sender_col),
            );
        }
        header = header.push(widget::text::caption(msg.timestamp.clone()));
        header = header.push(widget::horizontal_space());
        let reply_ctx = ReplyContext {
            event_id: msg.event_id.clone(),
            sender_id: msg.sender.clone(),
            sender_display: msg.sender_display.clone(),
            body_preview: msg.body.chars().take(80).collect(),
        };
        header = header.push(
            widget::button::text("↩")
                .on_press(Message::ReplyTo(reply_ctx))
                .padding([0, spacing.space_xxs]),
        );
        col = col.push(header);
    }

    // Render image or text body
    if msg.image.is_some() {
        if let Some(handle) = images.get(&msg.event_id) {
            col = col.push(
                cosmic::iced::widget::image(handle.clone())
                    .content_fit(ContentFit::Contain)
                    .width(Length::Fixed(400.0)),
            );
        } else {
            col = col.push(widget::text::caption("[Loading image...]"));
        }
        // Show filename as a subtle caption
        if !msg.body.is_empty() {
            col = col.push(widget::text::caption(msg.body.as_str()));
        }
    } else {
        col = col.push(widget::text::body(msg.body.clone()));
    }

    let top_pad = if msg.is_continuation && msg.reply_to_sender.is_none() {
        1
    } else {
        spacing.space_xxs
    };

    widget::container(col)
        .padding([top_pad, spacing.space_s])
        .width(Length::Fill)
        .into()
}
