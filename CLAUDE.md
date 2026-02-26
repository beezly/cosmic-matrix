# CLAUDE.md — Cosmic Matrix

## Project Overview

Cosmic Matrix is a native Matrix chat client built with [libcosmic](https://github.com/pop-os/libcosmic) (COSMIC's Rust UI toolkit, based on iced) and [matrix-sdk 0.9](https://github.com/matrix-org/matrix-rust-sdk).

## Architecture

Elm-style architecture: **State → Message → Update → View**

- All UI events flow through a single `Message` enum (`src/message.rs`)
- `App::update()` in `src/app.rs` is the single update handler
- Views are pure functions of state — no mutable widget state

```
src/
  app.rs          # Application impl, update(), view(), subscription()
  message.rs      # Message enum + shared data types (TimelineMessage, ImageContent, …)
  config.rs       # Session persistence (~/.config/cosmic-matrix/)
  colors.rs       # FNV-hashed per-sender colour palette
  matrix/
    client.rs     # Client creation, login, session restore
    sync.rs       # Sync subscription → RoomsUpdated / IncomingEvents
    timeline.rs   # SDK events → TimelineItem conversion
    verification.rs # Cross-signing bootstrap, SAS verification
  state/
    rooms.rs      # Room list with search filter + sorting
    timeline.rs   # Timeline items, pagination token, composer state
  ui/
    login.rs      # Login form
    timeline.rs   # Message list (inline images, replies, date separators)
    composer.rs   # Message input + attachment button
    room_header.rs
    verification.rs # Emoji SAS panel + incoming request banner
```

## Build & Run

```sh
cargo build          # debug
cargo run            # run debug
cargo build --release
just install         # install to /usr/local/bin (requires release build)
```

Logging is controlled via `RUST_LOG`, e.g. `RUST_LOG=cosmic_matrix=debug cargo run`.

## Key Conventions

### Adding a new feature
1. Add variant(s) to `Message` in `src/message.rs`
2. Add any new data types alongside (`src/message.rs` or a new file)
3. Handle the variant in `App::update()` in `src/app.rs`
4. Update view functions in `src/ui/`

### Async tasks
Use `cosmic::task::future(async move { … })` — returns a `Task<Message>`. Batch multiple tasks with `Task::batch(vec![…])`.

### libcosmic widget notes
- Text helpers: `widget::text::heading()`, `widget::text::body()`, `widget::text::caption()` — accept `Into<Cow<str>>`
- Custom-content buttons: `widget::button::custom(element).class(cosmic::theme::Button::…)`
- Image widget: `cosmic::iced::widget::image(handle).content_fit(ContentFit::Contain).width(Length::Fixed(px))`
- Image handles from bytes: `cosmic::iced::widget::image::Handle::from_bytes(vec_u8)` — wrap in `Arc` internally, cheap to clone

### matrix-sdk 0.9 notes
- Media fetch: `MediaRequestParameters { source: MediaSource, format: MediaFormat::File }` then `client.media().get_media_content(&req, use_cache)`
- `MediaSource` is at `matrix_sdk::ruma::events::room::MediaSource`
- Encryption handled transparently by the SDK for both `MediaSource::Plain` and `MediaSource::Encrypted`
- Verification: `client.encryption()` → `bootstrap_cross_signing_if_needed`, `cross_signing_status`, `get_user_identity`

### layout
`core.window.content_container = false` is set — `view()` must wrap its root in `widget::container(…).class(cosmic::theme::Container::Background)` to avoid a transparent window.
