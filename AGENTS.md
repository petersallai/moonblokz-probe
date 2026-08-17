# Repository Guidelines

## Project Structure & Module Organization
- `src/main.rs` starts the probe runtime and wires shared state.
- Key modules in `src/`: `usb_manager.rs` (single owner of USB serial), `usb_collector.rs` (log ingestion/filtering), `telemetry_sync.rs` (hub sync), `command_executor.rs` (remote commands), `update_manager.rs` (node/probe OTA flow).
- Config and deployment artifacts are in the repo root (`config.toml.example`, `moonblokz-probe.service`, `build.sh`, `quick_start.sh`).
- Version metadata and release binaries live in `versioninfo/probe/`.

## Build, Test, and Development Commands
- `cargo build --release`: build probe binary (`target/release/moonblokz-probe`).
- `cargo run -- --config config.toml`: run locally with explicit config.
- `cargo test`: run tests (currently zero tests; command must stay passing).
- `cargo fmt && cargo clippy --all-targets --all-features -D warnings`: format and lint gate before PR.
- `./build.sh [aarch64|arm|--no-increment]`: cross-build, compute CRC32, bump/update `version.json`, and copy versioned binary.

## Coding Style & Naming Conventions
- Rust 2021, `rustfmt` defaults, 4-space indentation.
- Naming: files/functions/variables `snake_case`, types `PascalCase`, constants `SCREAMING_SNAKE_CASE`.
- Prefer bounded, predictable behavior: avoid unbounded queues/allocations in hot paths.
- Use `log` macros (`info!`, `warn!`, `error!`) instead of `println!`.

## Protocol & Runtime Constraints
- Update flow is safety-critical: probe self-update uses versioned binary replacement + reboot; node update uses `/BS` bootloader command, mass-storage mount/copy, then reconnect.

## Testing Guidelines
- Prioritize tests for command parsing, scheduling, retry/backoff, CRC/version handling, and update failure recovery.
- Prefer deterministic tests over timing-fragile tests; use fixed inputs and explicit time windows.
- When behavior depends on network topology or relaying policy, validate with reproducible simulator scenarios (scene-driven, same protocol logic as embedded nodes).

## Commit & Pull Request Guidelines
- Commit messages should be imperative and scoped (e.g., `Refactor telemetry upload timing`).
- PRs must include: purpose, behavioral impact, config/ops impact, and verification commands run.
- If changing OTA, USB command semantics, or telemetry payloads, include rollback/risk notes and representative logs.

## Security & Configuration Tips
- Never commit secrets (`api_key`, internal endpoints); keep only examples in `config.toml.example`.
- Treat reboot/mount/umount paths as privileged operations; fail safely and log enough context for remote diagnosis.

## Further Reference
- MoonBlokz Series Part VII/5 (Field Testing Infrastructure): https://medium.com/moonblokz/moonblokz-series-part-vii-5-field-testing-infrastructure-6be10e18796c
