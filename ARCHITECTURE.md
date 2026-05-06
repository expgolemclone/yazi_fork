# Architecture

Yazi のフォーク。[sxyazi/yazi](https://github.com/sxyazi/yazi) をベースに favorites 機能と四季報向けの四半期移動を追加している。

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

### Dark theme 固定

配色 (preset) 選択で端末背景の自動判定を使わず、常に `theme-dark.toml` を採用する。端末エミュレータ検出自体は画像アダプタ等のために残し、`EMULATOR.light` も Lua の `ya.term.light` 経由で引き続き読める。テーマ選択だけを `yazi_config::effective_light()` (常に `false`) 経由に切り替えている。

反映箇所:
- 起動時: `yazi-adapter/src/lib.rs` → `init_flavor(effective_light())`
- Hot-reload: `yazi-actor/src/app/theme.rs` → `build_flavor(effective_light(), true)`
- `--debug`: `yazi-boot/src/actions/debug.rs` — "Detected background" (検出値) と "Effective theme mode: dark (fork override)" を表示

### Favorites トラバーサル

デフォルトの keymap に以下を追加している:

- `e` — カレントファイルのお気に入りトグル (`favorite`)
- `b` — 次のお気に入りへジャンプ (`favorite_arrow`)
- `B` — 前のお気に入りへジャンプ (`favorite_arrow --previous`)

b/B のトラバーサル順は JSON 登録順ではなく、パス名（= 銘柄コード）の昇順。

active な `b/B` 巡回中に `q/Q` で同一銘柄の別四半期 PDF へ移動した場合、favorite の前後候補は保持される。したがって次の `b/B` は `q/Q` 前にいた favorite を基準に続行される。一方で手動ホバー移動など通常の移動は従来どおり hovered を基準に再計算する。

関連ファイル:

- [yazi-actor/src/mgr/favorite_arrow.rs](../yazi-actor/src/mgr/favorite_arrow.rs) — `FavoriteArrow` アクター
- [yazi-actor/src/mgr/favorite.rs](../yazi-actor/src/mgr/favorite.rs) — `Favorite` アクター
- [yazi-core/src/mgr/favorites.rs](../yazi-core/src/mgr/favorites.rs) — `Favorites` コレクションとサイクルトラッキング（ソート済みトラバーサル）
- [yazi-shared/src/url/buf.rs](../yazi-shared/src/url/buf.rs) — `UrlBuf` に `Ord` / `PartialOrd` を追加
- [yazi-parser/src/mgr/favorite_arrow.rs](../yazi-parser/src/mgr/favorite_arrow.rs) — 引数パーサー
- [yazi-config/preset/keymap-default.toml](../yazi-config/preset/keymap-default.toml) — デフォルトキーマップ
- [state/favorites.json](../state/favorites.json) — お気に入りリスト（Git 管理の JSON 配列）

### 四季報の四半期移動

四季報 PDF (`japan_company_handbook/data/{YYYY_Q}/{ticker}.pdf`) をホバー中、同一銘柄の前後四半期へジャンプできる。

- `q` — 前四半期の同一銘柄 PDF へジャンプ (`quarter_arrow --previous`)
- `Q` — 次四半期の同一銘柄 PDF へジャンプ (`quarter_arrow`)

`QuarterArrow` は現在の `{data_root}/{YYYY_Q}/{ticker}.pdf` を解釈し、`data_root` 直下の実在ディレクトリを走査して前後の四半期を決める。欠番四半期は飛ばし、対応する PDF が無い場合だけ通知する。active な `b/B` 巡回中に `q/Q` を実行した場合は、その巡回状態を壊さずに current location だけ新しい四半期 PDF へ更新する。

関連ファイル:

- [yazi-actor/src/mgr/quarter_arrow.rs](../yazi-actor/src/mgr/quarter_arrow.rs) — `QuarterArrow` アクター、`b/B` 巡回連携、テスト
- [yazi-parser/src/mgr/quarter_arrow.rs](../yazi-parser/src/mgr/quarter_arrow.rs) — 引数パーサー
- [yazi-config/preset/keymap-default.toml](../yazi-config/preset/keymap-default.toml) — `q/Q` のデフォルト割り当て

## デプロイ

Nix flake (`~/nix-config/flake.nix`) 経由でローカルにデプロイする。pre-push hook がビルド・デプロイ・テストを自動実行する。

- [nix/](../nix) — Nix ビルド定義
- [scripts/](../scripts) — ビルド・バリデーションスクリプト
