# Architecture

Yazi のフォーク。[sxyazi/yazi](https://github.com/sxyazi/yazi) をベースに favorites 機能を追加している。

## クレート構成

Cargo ワークスペース (`Cargo.toml`) で管理される 30 のクレートからなる。

### エントリポイント

- [yazi-fm](../yazi-fm) — ファイルマネージャ本体（TUI バイナリ）
- [yazi-cli](../yazi-cli) — `ya` コマンドラインツール

### レイヤー

```
yazi-fm (executor)
  └── yazi-actor (Actor トレイトと各アクション実装)
        ├── yazi-core (状態: Manager, Tabs, Notify など)
        ├── yazi-parser (コマンド引数のパース)
        └── yazi-proxy (イベントディスパッチマクロ)

yazi-config (設定・プリセットの読み込み)
yazi-plugin (Lua プラグインシステム)
yazi-scheduler (非同期タスクスケジューラ)
yazi-binding (Lua ↔ Rust バインディング)
```

### 個別クレート

- [yazi-adapter](../yazi-adapter) — 画像表示アダプタ
- [yazi-boot](../yazi-boot) — 起動時初期化
- [yazi-codegen](../yazi-codegen) — コード生成
- [yazi-dds](../yazi-dds) — プロセス間データ配信
- [yazi-emulator](../yazi-emulator) — ターミナルエミュレータデータベース
- [yazi-ffi](../yazi-ffi) — FFI バインディング
- [yazi-fs](../yazi-fs) — ファイルシステムユーティリティ
- [yazi-macro](../yazi-macro) — 手続きマクロ (`act!`, `succ!` など)
- [yazi-packing](../yazi-packing) — パッキングユーティリティ
- [yazi-runner](../yazi-runner) — Lua ランナー
- [yazi-sftp](../yazi-sftp) — SFTP クライアント
- [yazi-shared](../yazi-shared) — 共有データ型 (`Url`, `Data` など)
- [yazi-shim](../yazi-shim) — クレート間 shim
- [yazi-term](../yazi-term) — ターミナル拡張
- [yazi-tty](../yazi-tty) — TTY アクセス層
- [yazi-vfs](../yazi-vfs) — 仮想ファイルシステム
- [yazi-watcher](../yazi-watcher) — ファイル変更監視
- [yazi-widgets](../yazi-widgets) — TUI ウィジェット

## Fork 固有の追加機能

### Favorites トラバーサル

デフォルトの keymap に以下を追加している:

- `e` — カレントファイルのお気に入りトグル (`favorite`)
- `b` — 次のお気に入りへジャンプ (`favorite_arrow`)
- `B` — 前のお気に入りへジャンプ (`favorite_arrow --previous`)

関連ファイル:

- [yazi-actor/src/mgr/favorite_arrow.rs](../yazi-actor/src/mgr/favorite_arrow.rs) — `FavoriteArrow` アクター
- [yazi-actor/src/mgr/favorite.rs](../yazi-actor/src/mgr/favorite.rs) — `Favorite` アクター
- [yazi-core/src/mgr/favorites.rs](../yazi-core/src/mgr/favorites.rs) — `Favorites` コレクションとサイクルトラッキング
- [yazi-parser/src/mgr/favorite_arrow.rs](../yazi-parser/src/mgr/favorite_arrow.rs) — 引数パーサー
- [yazi-config/preset/keymap-default.toml](../yazi-config/preset/keymap-default.toml) — デフォルトキーマップ
- [state/favorites.json](../state/favorites.json) — お気に入りリスト（Git 管理の JSON 配列）

## デプロイ

Nix flake (`~/nix-config/flake.nix`) 経由でローカルにデプロイする。pre-push hook がビルド・デプロイ・テストを自動実行する。

- [nix/](../nix) — Nix ビルド定義
- [scripts/](../scripts) — ビルド・バリデーションスクリプト
