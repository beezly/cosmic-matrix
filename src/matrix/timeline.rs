use std::collections::HashMap;

use matrix_sdk::room::MessagesOptions;
use matrix_sdk::ruma::events::room::message::MessageType;
use matrix_sdk::ruma::events::AnySyncTimelineEvent;
use matrix_sdk::{Room, RoomMemberships};

use crate::message::{ImageContent, TimelineItem, TimelineMessage};

pub async fn load_room_timeline(
    room: &Room,
) -> Result<(Vec<TimelineItem>, Option<String>), String> {
    let options = MessagesOptions::backward();
    let messages = room
        .messages(options)
        .await
        .map_err(|e| format!("Failed to load messages: {e}"))?;

    let display_names = build_display_names(room).await;

    let mut items = Vec::new();
    let mut last_date: Option<chrono::NaiveDate> = None;

    // Messages come in reverse order (newest first), so we reverse
    for event in messages.chunk.iter().rev() {
        if let Ok(ev) = event.raw().deserialize() {
            // Extract date for separator logic
            let ts_millis: i64 = ev.origin_server_ts().0.into();
            let item_date = ts_to_naive_date(ts_millis);

            if let Some(date) = item_date {
                if last_date.as_ref() != Some(&date) {
                    items.push(TimelineItem::DateSeparator(format_date_label(date)));
                    last_date = Some(date);
                }
            }

            match ev {
                AnySyncTimelineEvent::MessageLike(msg_ev) => {
                    if let Some(item) = convert_message_event(&msg_ev, &display_names) {
                        items.push(item);
                    }
                }
                AnySyncTimelineEvent::State(state_ev) => {
                    let desc = format_state_event(&state_ev);
                    if !desc.is_empty() {
                        items.push(TimelineItem::StateEvent(desc));
                    }
                }
            }
        }
    }

    apply_continuation_markers(&mut items);

    Ok((items, messages.end))
}

/// Fetch all joined members from the local store and return a user_id → display name map.
pub async fn build_display_names(room: &Room) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Ok(members) = room.members_no_sync(RoomMemberships::JOIN).await {
        for member in members {
            map.insert(member.user_id().to_string(), member.name().to_owned());
        }
    }
    map
}

fn strip_reply_fallback(body: &str) -> (Option<(String, String)>, String) {
    if !body.starts_with("> <@") {
        return (None, body.to_owned());
    }
    let (quote_block, real_body) = match body.find("\n\n") {
        Some(pos) => (&body[..pos], body[pos + 2..].to_owned()),
        None => return (None, body.to_owned()),
    };
    let first_line = quote_block.lines().next().unwrap_or("");
    let after_prefix = first_line.strip_prefix("> ").unwrap_or(first_line);

    let sender_id = after_prefix
        .strip_prefix('<')
        .and_then(|s| s.find('>').map(|i| s[..i].to_owned()))
        .unwrap_or_else(|| "@unknown".to_owned());

    let quoted_text = after_prefix
        .find('>')
        .map(|i| after_prefix[i + 1..].trim())
        .unwrap_or("");
    let preview: String = quoted_text.chars().take(80).collect();

    (Some((sender_id, preview)), real_body)
}

pub fn convert_message_event(
    event: &ruma::events::AnySyncMessageLikeEvent,
    display_names: &HashMap<String, String>,
) -> Option<TimelineItem> {
    use ruma::events::AnySyncMessageLikeEvent;

    match event {
        AnySyncMessageLikeEvent::RoomMessage(msg) => {
            let original = msg.as_original()?;
            let sender = original.sender.to_string();
            let sender_display = display_names
                .get(&sender)
                .cloned()
                .unwrap_or_else(|| original.sender.localpart().to_string());

            let ts_millis: i64 = original.origin_server_ts.0.into();
            let datetime =
                chrono::DateTime::from_timestamp_millis(ts_millis).unwrap_or_default();
            let time_str = datetime.format("%H:%M").to_string();

            let mut image_content: Option<ImageContent> = None;

            let (raw_body, is_emote) = match &original.content.msgtype {
                MessageType::Text(text) => (text.body.clone(), false),
                MessageType::Emote(emote) => (emote.body.clone(), true),
                MessageType::Notice(notice) => (notice.body.clone(), false),
                MessageType::Image(img) => {
                    image_content = Some(ImageContent {
                        source: img.source.clone(),
                    });
                    (img.body.clone(), false)
                }
                MessageType::File(_) => ("[File]".to_string(), false),
                MessageType::Audio(_) => ("[Audio]".to_string(), false),
                MessageType::Video(_) => ("[Video]".to_string(), false),
                _ => ("[Unsupported message type]".to_string(), false),
            };

            let (reply_ctx, body) = strip_reply_fallback(&raw_body);
            let (reply_to_sender, reply_to_body) = match reply_ctx {
                Some((id, preview)) => (Some(id), Some(preview)),
                None => (None, None),
            };

            let event_id = original.event_id.to_string();

            Some(TimelineItem::Message(TimelineMessage {
                event_id,
                sender,
                sender_display,
                body,
                timestamp: time_str,
                is_emote,
                is_continuation: false,
                reply_to_sender,
                reply_to_body,
                image: image_content,
            }))
        }
        AnySyncMessageLikeEvent::RoomEncrypted(_) => {
            Some(TimelineItem::Message(TimelineMessage {
                event_id: String::new(),
                sender: String::new(),
                sender_display: String::new(),
                body: "[Unable to decrypt]".to_string(),
                timestamp: String::new(),
                is_emote: false,
                is_continuation: false,
                reply_to_sender: None,
                reply_to_body: None,
                image: None,
            }))
        }
        _ => None,
    }
}

pub fn ts_to_naive_date(ts_millis: i64) -> Option<chrono::NaiveDate> {
    chrono::DateTime::from_timestamp_millis(ts_millis).map(|dt| dt.date_naive())
}

pub fn format_date_label(date: chrono::NaiveDate) -> String {
    let today = chrono::Local::now().date_naive();
    if date == today {
        "Today".to_string()
    } else if date == today - chrono::Duration::days(1) {
        "Yesterday".to_string()
    } else {
        date.format("%B %d, %Y").to_string()
    }
}

/// Set `is_continuation = true` on consecutive messages from the same sender.
/// A DateSeparator or StateEvent resets the grouping.
pub fn apply_continuation_markers(items: &mut Vec<TimelineItem>) {
    let mut last_sender: Option<String> = None;
    for item in items.iter_mut() {
        match item {
            TimelineItem::Message(ref mut msg) => {
                msg.is_continuation = last_sender.as_deref() == Some(&msg.sender);
                last_sender = Some(msg.sender.clone());
            }
            _ => {
                last_sender = None;
            }
        }
    }
}

/// Remove consecutive DateSeparator items with the same label (dedup after prepend).
pub fn dedup_adjacent_date_separators(items: &mut Vec<TimelineItem>) {
    let mut i = 0;
    while i + 1 < items.len() {
        let is_dup = matches!(
            (&items[i], &items[i + 1]),
            (TimelineItem::DateSeparator(a), TimelineItem::DateSeparator(b)) if a == b
        );
        if is_dup {
            items.remove(i);
        } else {
            i += 1;
        }
    }
}

fn format_state_event(event: &ruma::events::AnySyncStateEvent) -> String {
    use ruma::events::AnySyncStateEvent;
    match event {
        AnySyncStateEvent::RoomMember(ev) => {
            if let Some(original) = ev.as_original() {
                let user = original.state_key.to_string();
                match original.content.membership {
                    ruma::events::room::member::MembershipState::Join => {
                        format!("{user} joined the room")
                    }
                    ruma::events::room::member::MembershipState::Leave => {
                        format!("{user} left the room")
                    }
                    _ => String::new(),
                }
            } else {
                String::new()
            }
        }
        AnySyncStateEvent::RoomName(ev) => {
            if let Some(original) = ev.as_original() {
                format!("Room name changed to: {}", &original.content.name)
            } else {
                String::new()
            }
        }
        AnySyncStateEvent::RoomTopic(ev) => {
            if let Some(original) = ev.as_original() {
                format!("Topic changed to: {}", original.content.topic)
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}
