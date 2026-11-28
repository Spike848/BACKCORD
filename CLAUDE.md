# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

@AGENTS.md

The rules and commands above, from `AGENTS.md`, apply in full. What follows adds to them.

## Commands

Every cargo command runs inside the dev shell: `nix develop path:./nix --command bash -c '...'`.

- **Format:** `cargo fmt --all`. `scripts/check` fails on unformatted code.
- **Lint one crate:** `cargo clippy --profile fast -p backcord-core --all-targets`. `scripts/check` denies warnings, including the extra lints in `[workspace.lints]` of `Cargo.toml`. An `#[allow]` needs `reason = "..."`.
- **Run one test, or every test whose name contains a word:** `cargo test --profile fast -p backcord-core --lib voice_compressor`. Swap the crate with `-p backcord-iced`, `-p backcord-discord` or `-p backcord-iced-kit`.
- **Full check before the last commit:** `scripts/check --fast`. Add `--matrix` when code shared with Matrix changes.
- **Discord service tests:** `scripts/test-fake-discord [FILTER]`. They run against `testing/fake_discord.ts`, and `scripts/check` doesn't run them.
- **Try the UI with no account:** `cargo run --locked --profile fast -- --preview`. The preview has local sample data and makes no network calls.
- **Windows release exe:** `unset LK_CUSTOM_WEBRTC; scripts/build-windows discord`, run from Linux or WSL through cargo-xwin. The exe lands in `target/x86_64-pc-windows-msvc/release/BACKCORD.exe`.
- **Version bump:** `scripts/set-version X.Y.Z`, before the "BACKCORD X.Y.Z" commit and its `vX.Y.Z` tag.

## Architecture in brief

`docs/architecture.md` is the full map. These are the parts you need to see across files:

- **One loop through the app:**
  1. The window (`crates/backcord-iced`) sends an `Action` to `Application::dispatch`, defined in `crates/backcord-core/src/application.rs`. That file routes each action to the module for its area under `application/`.
  2. `dispatch` returns `Effect`s. `Effect::Backend(Command::...)` goes to a network backend: `discord_text/commands.rs` for Discord, `matrix_chat.rs` for Matrix.
  3. The backend answers with `Event`s, which `Application::apply` takes in.
  4. The window reads `AppState` and takes in only what changed (`sync.rs`).

  `Command` and `Event` live in `crates/backcord-core/src/chat.rs`. So does `ChatMessage`, the row the timeline draws.
- **Discord, end to end:**
  - `crates/backcord-discord` parses Discord's JSON. Message parsing is in `message.rs`, the HTTP API in `api.rs`, the gateway state in `service.rs` and `service/`.
  - An open channel lives in `backcord-core/src/discord_text/open_channel.rs`, which turns Discord messages into `ChatMessage`s (`render`).
  - Background lookups finish as `discord_text::Done` variants, which `commands.rs` handles.
  - A new field on a message usually touches all four: the parser, `ChatMessage`, `render` and the view.
- **Window modules:** each area of `crates/backcord-iced/src` has its own `Event`, `update` and `view`, and the view only reads.
  - Right-click menus are built in `menus.rs` (messages), `person_menu.rs`, `place_menus.rs` and `picks.rs` (people picked with Ctrl). Messages picked with Ctrl are handled in `timeline/picks.rs`.
  - Settings are saved per field in `layout.rs` and applied at startup in `play_volumes`.
- **Audio settings** are process-wide atomics in `backcord-core/src/volume.rs`, which the audio threads read on every frame.
  - People's voices go through `discord_audio.rs`: high-pass, gate, compressor, limiter, pitch.
  - The microphone's stereo effects are in `backcord-discord/src/spatial.rs`. Opus settings are in `call_connection.rs` and `opus.rs`.

## Interface text (Italian)

- **Every label goes through a translation call.** Fixed text uses `backcord_iced_kit::i18n::tr("English text")`. Text built at run time uses `i18n::t(format!(...))`, with `{}` placeholders in the table entry.
- **Add each new string to the table.** The translations live in `crates/backcord-iced-kit/src/i18n.tsv`, one `English<TAB>Italian` line per string. A string with no entry shows in English.
- **Tests see English.** The window's tests build with the `english` feature, so they look for the English text: `shows(&app, "...")` in `crates/backcord-iced/src/tests/`.
- **Keep terms consistent.** Reuse the Italian an existing entry already gives for a shared word; for example, Light/Medium/Full are "Minima/Media/Massima".
