# Cosmic Matrix

A native [Matrix](https://matrix.org) chat client for the [COSMIC](https://system76.com/cosmic) desktop environment, built in Rust with [libcosmic](https://github.com/pop-os/libcosmic) and [matrix-rust-sdk](https://github.com/matrix-org/matrix-rust-sdk).

## Features

- **Login & session restore** — standard Matrix password login with persistent session storage
- **Room list** — searchable, sorted by recent activity with unread counts
- **Timeline** — message history with pagination, date separators, and grouped messages from the same sender
- **Inline images** — `m.image` events rendered directly in the timeline (fetched via the SDK, encrypted images supported)
- **Replies** — send and display threaded replies with quoted preview
- **File attachments** — send files via the native COSMIC file picker
- **End-to-end encryption** — E2EE via matrix-sdk (SQLite session store)
- **Cross-signing** — bootstrap cross-signing keys on login, verify other devices via interactive SAS emoji verification
- **COSMIC theming** — adapts to system light/dark theme automatically

## Screenshots

> Coming soon

## Requirements

- Rust 1.78 or later
- A COSMIC or compatible Wayland compositor (also runs under X11 via XWayland)
- A Matrix account

## Building

```sh
git clone https://github.com/beezly/cosmic-matrix
cd cosmic-matrix
cargo build --release
```

## Running

```sh
cargo run                        # debug build
RUST_LOG=cosmic_matrix=debug cargo run  # with debug logging
```

Or after `cargo build --release`:

```sh
./target/release/cosmic-matrix
```

## Installing

Requires [just](https://github.com/casey/just):

```sh
cargo build --release
just install
```

This installs the binary to `/usr/local/bin/cosmic-matrix` and the `.desktop` / metainfo files to the appropriate system paths.

## Configuration

Session credentials are stored at `~/.config/cosmic-matrix/session.json`. Delete this file to log out and clear the saved session.

## Project Structure

See [CLAUDE.md](CLAUDE.md) for a detailed architecture overview aimed at AI assistants and contributors.

## License

GPL-3.0 — see [Cargo.toml](Cargo.toml).
