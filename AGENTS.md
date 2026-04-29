# Repository Instructions

- When a repo change affects the user-facing installed behavior, carry the change through the required local Nix/Home Manager/NixOS reflection steps without pausing to ask for confirmation.
- Treat rebuilding or switching the local Nix-managed environment as part of completing the task when that is necessary for the user's normal `yazi` invocation to pick up the repo change.
- For test-only setup that touches process-global state, prefer `yazi_shared::init_tests()` and `yazi_fs::init_tests()` instead of calling the raw `init()` functions from multiple tests in the same binary.
- Before pushing repository changes, verify with `cargo test --workspace --verbose` unless the task explicitly requires a different validation path.
- Git-managed favorites live at `state/favorites.json`, and the local deployment points `mgr.favorites_file` there via `~/nix-config/home/yazi/yazi.toml`.
