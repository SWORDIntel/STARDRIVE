# Repository Guidelines

## Project Structure & Module Organization
- `displaylink-driver/` is the Rust crate you ship; `src/` contains the binary logic, `tests/` holds integration suites, and `Cargo.toml` describes dependencies.
- `evdi_source/library` builds the user-space helpers while `evdi_source/module` provides the kernel piece; both must be installed before `displaylink-driver` can claim screens.
- Keep reference material in `docs/`, `reference/`, and the top-level manuals (`README.md`, `BUILD.md`, `PROTOCOL.md`, `PHASE6.md`) so they stay in sync with the code changes.

## Build, Test, and Development Commands
- Follow `BUILD.md` for prerequisites, then run `cd evdi_source/library && make && sudo make install` and `cd ../module && sudo make install && sudo modprobe evdi` (swap in the DKMS block if you prefer automatic rebuilds).
- `cd displaylink-driver && cargo build --release` produces `target/release/displaylink-driver`; run the binary with elevated rights unless you have already added the udev rule for `17e9:4307`.
- `cargo test` validates the Rust logic; add `-- --nocapture` to surfacing logs when you investigate failures.

## Coding Style & Naming Conventions
- Adopt Rust 2021 defaults: 4-space indentation, `snake_case` for functions/variables, `CamelCase` for structs/enums, and upper snake for constants.
- Let module names match directories (e.g., `src/usb.rs` → `usb` module) so Cargo’s resolution stays predictable.
- Run `cargo fmt` regularly and leave lingering `clippy` warnings documented if you push before they sink.

## Testing Guidelines
- The tests under `displaylink-driver/tests` are integration-focused; name files after the feature they validate (`protocol.rs`, `compression.rs`, etc.).
- Each `#[test]` should clean up acquired resources and stay deterministic; add helper fixtures when multiple tests repeat the same setup.
- Combine unit tests with the manual verification steps in `README.md` (`lsusb`, `xrandr`) when validating a hardware-facing change.

## Commit & Pull Request Guidelines
- Commit messages follow an imperative, descriptive style (`Implement Phase 6 RLE tweaks`, `Document new EVDI warning`); reference the phase or issue when it helps reviewers understand context.
- PRs need a short summary, explicit test steps, linked issues or discussions, and notes about manual steps such as installing EVDI or reloading `udev` rules; include screenshots only when the change alters user-visible output.

## Security & Configuration Tips
- The driver touches kernel modules and USB devices, so minimize `sudo` usage and explain any privileged commands in your PR.
- Limit udev rules to the `17e9:4307` pair and reload them immediately (`udevadm control --reload-rules && udevadm trigger`) before running the driver as a normal user.
