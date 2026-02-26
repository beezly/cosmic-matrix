use std::sync::Arc;

use cosmic::iced::futures::SinkExt;
use cosmic::iced::stream;
use cosmic::iced::Subscription;
use matrix_sdk::config::SyncSettings;
use matrix_sdk::ruma::api::client::filter::FilterDefinition;
use matrix_sdk::ruma::events::{AnySyncTimelineEvent, AnyToDeviceEvent};
use matrix_sdk::Client;

use crate::matrix::timeline::convert_message_event;
use crate::message::{Message, RoomEntry, TimelineItem};

pub fn sync_subscription(client: Arc<Client>) -> Subscription<Message> {
    Subscription::run_with_id(
        std::any::TypeId::of::<SyncSubscriptionMarker>(),
        stream::channel(100, move |mut output| {
            let client = client.clone();
            async move {
                let _ = output.send(Message::SyncStarted).await;

                let filter = FilterDefinition::with_lazy_loading();
                let settings = SyncSettings::default().filter(filter.into());

                // Run initial sync and collect rooms
                match client.sync_once(settings.clone()).await {
                    Ok(response) => {
                        let rooms = collect_rooms(&client).await;
                        let _ = output.send(Message::RoomsUpdated(rooms)).await;

                        // Emit incoming events for the initial sync
                        for (room_id, update) in &response.rooms.join {
                            let new_items = extract_new_items_from_events(
                                &client, room_id, &update.timeline.events,
                            ).await;
                            if !new_items.is_empty() {
                                let _ = output
                                    .send(Message::IncomingEvents(room_id.clone(), new_items))
                                    .await;
                            }
                        }

                        // Emit incoming to-device verification requests
                        emit_verification_requests(&response.to_device, &mut output).await;

                        // Continue syncing
                        let mut settings = settings.token(response.next_batch);
                        loop {
                            match client.sync_once(settings.clone()).await {
                                Ok(response) => {
                                    settings = settings.token(response.next_batch);
                                    let rooms = collect_rooms(&client).await;
                                    let _ = output.send(Message::RoomsUpdated(rooms)).await;

                                    for (room_id, update) in &response.rooms.join {
                                        let new_items = extract_new_items_from_events(
                                            &client, room_id, &update.timeline.events,
                                        ).await;
                                        if !new_items.is_empty() {
                                            let _ = output
                                                .send(Message::IncomingEvents(
                                                    room_id.clone(),
                                                    new_items,
                                                ))
                                                .await;
                                        }
                                    }

                                    emit_verification_requests(&response.to_device, &mut output)
                                        .await;
                                }
                                Err(e) => {
                                    let _ = output
                                        .send(Message::SyncError(format!("Sync error: {e}")))
                                        .await;
                                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = output
                            .send(Message::SyncError(format!("Initial sync failed: {e}")))
                            .await;
                    }
                }

                // Keep subscription alive
                futures::future::pending::<()>().await;
            }
        }),
    )
}

async fn extract_new_items_from_events(
    client: &Client,
    room_id: &matrix_sdk::ruma::OwnedRoomId,
    events: &[matrix_sdk::deserialized_responses::SyncTimelineEvent],
) -> Vec<TimelineItem> {
    let display_names = if let Some(room) = client.get_room(room_id) {
        crate::matrix::timeline::build_display_names(&room).await
    } else {
        std::collections::HashMap::new()
    };

    let mut items = Vec::new();
    for ev in events {
        if let Ok(AnySyncTimelineEvent::MessageLike(msg_ev)) = ev.raw().deserialize() {
            if let Some(item) = convert_message_event(&msg_ev, &display_names) {
                items.push(item);
            }
        }
    }
    items
}

async fn collect_rooms(client: &Client) -> Vec<RoomEntry> {
    let mut entries = Vec::new();

    for room in client.joined_rooms() {
        let name = room
            .cached_display_name()
            .map(|n| n.to_string())
            .unwrap_or_else(|| room.room_id().to_string());

        let counts = room.unread_notification_counts();

        let is_encrypted = room.is_encrypted().await.unwrap_or(false);

        let topic = room.topic();

        let avatar_letter = name.chars().next().unwrap_or('#');

        let (last_message, last_message_ts) = room
            .latest_event()
            .and_then(|ev| {
                let timeline_ev = ev.event().raw().deserialize().ok()?;
                let ts_millis: i64 = timeline_ev.origin_server_ts().0.into();
                if let AnySyncTimelineEvent::MessageLike(ref msg_ev) = timeline_ev {
                    // Use empty map here — sidebar previews don't need resolved names
                    if let Some(TimelineItem::Message(m)) =
                        convert_message_event(msg_ev, &std::collections::HashMap::new())
                    {
                        return Some((Some(m.body), Some(ts_millis as u64)));
                    }
                }
                None
            })
            .unwrap_or((None, None));

        entries.push(RoomEntry {
            room_id: room.room_id().to_owned(),
            name,
            unread_count: counts.notification_count,
            is_encrypted,
            topic,
            last_message,
            last_message_ts,
            avatar_letter,
        });
    }

    // Sort: unread first, then most recent activity, then alphabetically
    entries.sort_by(|a, b| {
        b.unread_count
            .cmp(&a.unread_count)
            .then_with(|| b.last_message_ts.cmp(&a.last_message_ts))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    entries
}

async fn emit_verification_requests(
    to_device: &[matrix_sdk::ruma::serde::Raw<AnyToDeviceEvent>],
    output: &mut cosmic::iced::futures::channel::mpsc::Sender<Message>,
) {
    for raw_ev in to_device {
        if let Ok(AnyToDeviceEvent::KeyVerificationRequest(ev)) = raw_ev.deserialize() {
            let _ = output
                .send(Message::IncomingVerificationRequest {
                    flow_id: ev.content.transaction_id.to_string(),
                    sender: ev.sender.to_string(),
                })
                .await;
        }
    }
}

struct SyncSubscriptionMarker;
