use cosmic::iced::widget::scrollable::RelativeOffset;
use matrix_sdk::ruma::events::room::MediaSource;
use matrix_sdk::ruma::{OwnedRoomId, OwnedUserId};
use matrix_sdk::Client;

use crate::config::SortMode;

/// Wrapper for matrix_sdk::Client that implements Debug.
#[derive(Clone)]
pub struct MatrixClient(pub Client);

impl std::fmt::Debug for MatrixClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MatrixClient(..)")
    }
}

// ---- Verification types ----

#[derive(Clone, Debug)]
pub enum VerificationPhase {
    WaitingForAccept,
    SasStarted,
    ShowingEmoji(Vec<(String, String)>), // (symbol, description) × 7
    Confirming,
    Done,
    Cancelled(String),
}

#[derive(Clone, Debug)]
pub struct VerificationInfo {
    pub flow_id: String,
    pub other_user_id: String,
    pub phase: VerificationPhase,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CrossSigningStatus {
    Unknown,
    Verified,
    Unverified,
}

#[derive(Clone, Debug)]
pub enum VerificationStateUpdate {
    Accepted,
    EmojiReady(Vec<(String, String)>),
    Done,
    Cancelled(String),
}

// ---- Reply context ----

#[derive(Clone, Debug)]
pub struct ReplyContext {
    pub event_id: String,
    pub sender_id: String,
    pub sender_display: String,
    pub body_preview: String,
}

// ---- Core app messages ----

#[derive(Clone, Debug)]
pub enum Message {
    // -- Lifecycle --
    None,

    // -- Login --
    HomeserverChanged(String),
    UsernameChanged(String),
    PasswordChanged(String),
    TogglePasswordVisibility,
    LoginSubmit,
    LoginResult(Result<(MatrixClient, LoginSuccess), String>),
    SessionRestored(MatrixClient),
    Logout,

    // -- Sync --
    SyncStarted,
    RoomsUpdated(Vec<RoomEntry>),
    SyncError(String),

    // -- Room list --
    SelectRoom(OwnedRoomId),
    RoomFilterChanged(String),
    SetSortMode(SortMode),
    ToggleFavourite(OwnedRoomId),
    FavouriteToggled(OwnedRoomId, bool),
    ToggleSection(String), // section key

    // -- Timeline --
    TimelineUpdated(OwnedRoomId, Vec<TimelineItem>, Option<String>),
    IncomingEvents(OwnedRoomId, Vec<TimelineItem>),
    ComposerChanged(String),
    SendMessage,
    MessageSent(OwnedRoomId),
    SendError(String),
    LoadMoreHistory,
    HistoryLoaded(OwnedRoomId, Vec<TimelineItem>, Option<String>),
    TimelineScrolled(RelativeOffset),
    ScrollToBottom,

    // -- Reply --
    ReplyTo(ReplyContext),
    CancelReply,

    // -- Attachments --
    PickAttachment,
    AttachmentSent(OwnedRoomId),
    AttachmentError(String),

    // -- Inline images --
    ImageFetched { event_id: String, data: Vec<u8> },
    ImageFetchFailed { event_id: String },

    // -- Cross-signing bootstrap --
    BootstrapCrossSigning,
    CrossSigningBootstrapped,
    CrossSigningBootstrapFailed(String),
    CrossSigningStatusFetched(CrossSigningStatus),

    // -- Outgoing self-verification --
    StartVerification,
    VerificationRequestCreated(String), // flow_id

    // -- Incoming verification --
    IncomingVerificationRequest { flow_id: String, sender: String },
    AcceptVerification,
    IgnoreVerification,

    // -- Subscription-driven state --
    VerificationStateChanged(VerificationStateUpdate),

    // -- User actions on emoji panel --
    VerificationConfirm,
    VerificationMismatch,
    CancelVerification,
}

#[derive(Clone, Debug)]
pub struct LoginSuccess {
    pub user_id: OwnedUserId,
    pub device_id: String,
}

#[derive(Clone, Debug)]
pub struct RoomEntry {
    pub room_id: OwnedRoomId,
    pub name: String,
    /// Total unread notification count.
    pub unread_count: u64,
    /// Highlight/mention count (subset of unread_count).
    pub mention_count: u64,
    pub is_encrypted: bool,
    pub topic: Option<String>,
    pub last_message: Option<String>,
    pub last_message_ts: Option<u64>,
    pub avatar_letter: char,
    /// Room has the m.favourite Matrix tag.
    pub is_favourite: bool,
    /// Room has the m.lowpriority Matrix tag.
    pub is_low_priority: bool,
    /// Room is a direct message (appears in m.direct account data).
    pub is_dm: bool,
}

#[derive(Clone, Debug)]
pub enum TimelineItem {
    Message(TimelineMessage),
    DateSeparator(String),
    StateEvent(String),
    UnreadMarker,
}

/// Metadata for an image message. The image bytes are fetched separately.
#[derive(Clone, Debug)]
pub struct ImageContent {
    pub source: MediaSource,
}

#[derive(Clone, Debug)]
pub struct TimelineMessage {
    pub event_id: String,
    pub sender: String,
    pub sender_display: String,
    /// Body text (or image filename for image messages).
    pub body: String,
    pub timestamp: String,
    pub is_emote: bool,
    pub is_continuation: bool,
    pub reply_to_sender: Option<String>,
    pub reply_to_body: Option<String>,
    /// Present when this message is an image (m.image).
    pub image: Option<ImageContent>,
}
