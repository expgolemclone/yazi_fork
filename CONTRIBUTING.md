# Contributing

## Verify first

Run the full workspace test suite before pushing:

```sh
cargo test --workspace --verbose
```

## Reflect installed behavior

This fork is consumed by `~/nix-config` as a local flake input overlay. If a change affects the installed user-facing behavior, rebuild and switch the local Nix-managed environment so the normal `yazi` / `ya` invocation picks it up.

Examples of user-facing changes:

- default keybindings
- presets
- runtime config defaults
- behavior visible in the installed TUI

Apply the local deployment after verification:

```sh
nixos-rebuild build --flake ~/nix-config#nixos --override-input yazi-fork /home/exp/projects/yazi_fork
sudo -n nixos-rebuild switch --flake ~/nix-config#nixos --override-input yazi-fork /home/exp/projects/yazi_fork
yazi --version
ya --version
```

Use the override because a plain `nixos-rebuild` may keep using the older locked `path` input from `~/nix-config/flake.lock`.

## Favorites in this fork

Git-managed favorites live at `state/favorites.json`, and the local deployment points `mgr.favorites_file` there via `~/nix-config/home/yazi/yazi.toml`.
