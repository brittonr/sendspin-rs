# Repository Guidelines

## Project Structure & Modules
- `src/` – core library code. Key modules: `protocol/` (WebSocket messaging and client), `audio/` (audio types, buffer pool), `scheduler/` (timed playback), `sync/` (clock sync utilities), `lib.rs` (public exports).
- `tests/` – integration tests covering protocol messages, scheduler, buffer pool, clock sync, PCM decoding, and audio output paths.
- `examples/` – runnable samples (e.g., `basic_client`) demonstrating client usage.
- `docs/` – deeper design/architecture notes (see `docs/rust-thoughts.md`).

## Build, Test, and Development Commands
- `cargo build` – standard debug build.
- `cargo test` – run integration tests; add `RUST_LOG=debug` for verbose output.
- `cargo run --example basic_client` – run the basic client example against a server.
- `cargo build --release` – optimized build for performance benchmarking.
- `cargo fmt` / `cargo clippy --all-targets -- -D warnings` – format and lint before opening a PR.

## Coding Style & Naming Conventions
- Follow Rust defaults: snake_case for functions/modules, PascalCase for types/traits, SCREAMING_SNAKE_CASE for constants and env keys.
- Keep modules focused; prefer small files per concern (e.g., audio types vs. buffer pool).
- Favor zero-copy and lock-free patterns consistent with the project goals; avoid unnecessary allocations on hot paths.
- Document public items (`#![warn(missing_docs)]` active); add concise `///` docs for new APIs.

## Testing Guidelines
- Co-locate integration coverage in `tests/`; mirror module names (e.g., `protocol_messages.rs`).
- For new behaviors, add targeted tests rather than broad integration blasts; prefer deterministic async tests (Tokio) over timing-sensitive ones.
- Run `cargo test` locally before pushing; include any feature flags or env vars used in the PR description.

## Commit & Pull Request Guidelines
- Commits: short imperative summaries (per git history), scope-limited changes, and tidy diffs. Squash fixups before merging.
- PRs: include a brief description, linked issues, rationale for design choices, and test/format results (`cargo test`, `cargo fmt`, `cargo clippy`). Add logs or screenshots if touching runtime behavior.

## Security & Configuration Tips
- Avoid checking secrets; prefer env vars for endpoints/keys. Do not commit generated credentials or captures.
- Validate external inputs in protocol-facing code; bubble errors through `error::Error` variants for clarity.
