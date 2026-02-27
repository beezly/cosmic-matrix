use matrix_sdk::ruma::OwnedRoomId;

use crate::config::SortMode;
use crate::message::RoomEntry;

pub const SECTION_FAVOURITES: &str = "favourites";
pub const SECTION_DMS: &str = "dms";
pub const SECTION_ROOMS: &str = "rooms";
pub const SECTION_LOW_PRIORITY: &str = "low_priority";

/// A section of the room list.
#[derive(Debug, Clone)]
pub struct RoomSection {
    pub key: &'static str,
    pub label: &'static str,
    pub collapsed: bool,
    pub rooms: Vec<OwnedRoomId>,
}

pub struct RoomsState {
    pub rooms: Vec<RoomEntry>,
    pub selected: Option<OwnedRoomId>,
    pub filter: String,
    pub sort_mode: SortMode,
    /// section key → collapsed
    pub sections_collapsed: std::collections::HashMap<String, bool>,
}

impl Default for RoomsState {
    fn default() -> Self {
        Self {
            rooms: Vec::new(),
            selected: None,
            filter: String::new(),
            sort_mode: SortMode::default(),
            sections_collapsed: std::collections::HashMap::new(),
        }
    }
}

impl RoomsState {
    pub fn update_rooms(&mut self, rooms: Vec<RoomEntry>) {
        self.rooms = rooms;
    }

    pub fn is_section_collapsed(&self, key: &str) -> bool {
        self.sections_collapsed.get(key).copied().unwrap_or(false)
    }

    pub fn toggle_section(&mut self, key: &str) {
        let current = self.is_section_collapsed(key);
        self.sections_collapsed.insert(key.to_string(), !current);
    }

    /// Return rooms matching the current filter, separated into labelled sections.
    /// Sorting within each section respects `self.sort_mode`.
    pub fn sections(&self) -> Vec<RoomSection> {
        let query = if self.filter.is_empty() {
            None
        } else {
            Some(self.filter.to_lowercase())
        };

        let mut favs: Vec<&RoomEntry> = Vec::new();
        let mut dms: Vec<&RoomEntry> = Vec::new();
        let mut rooms: Vec<&RoomEntry> = Vec::new();
        let mut low: Vec<&RoomEntry> = Vec::new();

        for room in &self.rooms {
            if let Some(ref q) = query {
                if !room.name.to_lowercase().contains(q.as_str()) {
                    continue;
                }
            }
            if room.is_favourite {
                favs.push(room);
            } else if room.is_low_priority {
                low.push(room);
            } else if room.is_dm {
                dms.push(room);
            } else {
                rooms.push(room);
            }
        }

        let sort_fn = |a: &&RoomEntry, b: &&RoomEntry| -> std::cmp::Ordering {
            // Within any section, unread rooms sort to the top
            let a_unread = a.unread_count > 0 || a.mention_count > 0;
            let b_unread = b.unread_count > 0 || b.mention_count > 0;
            if a_unread != b_unread {
                return b_unread.cmp(&a_unread);
            }
            match self.sort_mode {
                SortMode::Alphabetical => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortMode::RecentActivity => b
                    .last_message_ts
                    .cmp(&a.last_message_ts)
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
            }
        };

        favs.sort_by(sort_fn);
        dms.sort_by(sort_fn);
        rooms.sort_by(sort_fn);
        low.sort_by(sort_fn);

        let to_section = |key: &'static str, label: &'static str, list: Vec<&RoomEntry>| {
            RoomSection {
                key,
                label,
                collapsed: self.is_section_collapsed(key),
                rooms: list.into_iter().map(|r| r.room_id.clone()).collect(),
            }
        };

        let mut sections = Vec::new();
        if !favs.is_empty() {
            sections.push(to_section(SECTION_FAVOURITES, "Favourites", favs));
        }
        if !dms.is_empty() {
            sections.push(to_section(SECTION_DMS, "Direct Messages", dms));
        }
        if !rooms.is_empty() {
            sections.push(to_section(SECTION_ROOMS, "Rooms", rooms));
        }
        if !low.is_empty() {
            sections.push(to_section(SECTION_LOW_PRIORITY, "Low Priority", low));
        }
        sections
    }

    /// Flat filtered list (used for search + simple iteration).
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
