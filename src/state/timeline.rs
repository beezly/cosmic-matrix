use matrix_sdk::ruma::OwnedRoomId;

use crate::message::{ReplyContext, TimelineItem};

pub struct TimelineState {
    pub room_id: Option<OwnedRoomId>,
    pub items: Vec<TimelineItem>,
    pub composer: String,
    pub pagination_token: Option<String>,
    pub loading: bool,
    pub sending: bool,
    pub attachment_sending: bool,
    pub at_bottom: bool,
    pub unread_marker_inserted: bool,
    pub reply_to: Option<ReplyContext>,
}

impl Default for TimelineState {
    fn default() -> Self {
        Self {
            room_id: None,
            items: Vec::new(),
            composer: String::new(),
            pagination_token: None,
            loading: false,
            sending: false,
            attachment_sending: false,
            at_bottom: true,
            unread_marker_inserted: false,
            reply_to: None,
        }
    }
}

impl TimelineState {
    pub fn clear(&mut self) {
        self.room_id = None;
        self.items.clear();
        self.composer.clear();
        self.pagination_token = None;
        self.loading = false;
        self.sending = false;
        self.attachment_sending = false;
        self.at_bottom = true;
        self.unread_marker_inserted = false;
        self.reply_to = None;
    }

    pub fn set_timeline(&mut self, room_id: OwnedRoomId, items: Vec<TimelineItem>, token: Option<String>) {
        self.room_id = Some(room_id);
        self.items = items;
        self.pagination_token = token;
        self.loading = false;
        self.at_bottom = true;
        self.unread_marker_inserted = false;
        self.reply_to = None;
    }

    pub fn prepend_items(&mut self, mut items: Vec<TimelineItem>, token: Option<String>) {
        items.append(&mut self.items);
        self.items = items;
        self.pagination_token = token;
        self.loading = false;
    }
}
