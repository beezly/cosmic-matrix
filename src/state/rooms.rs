use matrix_sdk::ruma::OwnedRoomId;

use crate::message::RoomEntry;

#[derive(Default)]
pub struct RoomsState {
    pub rooms: Vec<RoomEntry>,
    pub selected: Option<OwnedRoomId>,
    pub filter: String,
}

impl RoomsState {
    pub fn update_rooms(&mut self, rooms: Vec<RoomEntry>) {
        self.rooms = rooms;
    }

    pub fn filtered_rooms(&self) -> Vec<&RoomEntry> {
        if self.filter.is_empty() {
            self.rooms.iter().collect()
        } else {
            let query = self.filter.to_lowercase();
            self.rooms
                .iter()
                .filter(|r| r.name.to_lowercase().contains(&query))
                .collect()
        }
    }

    pub fn selected_room_name(&self) -> Option<&str> {
        self.selected.as_ref().and_then(|sel| {
            self.rooms
                .iter()
                .find(|r| &r.room_id == sel)
                .map(|r| r.name.as_str())
        })
    }
}
