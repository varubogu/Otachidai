# お立ち台Bot

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![English](https://img.shields.io/badge/README-English-blue)](README.md)

お立ち台Botは、Discordサーバーに用意された部屋をレンタル申請して利用するためのBotです。
ユーザーは申請→利用→解放のフローで部屋を使えます。

## 機能一覧

- **チャンネルレンタル管理** — 申請・承認・解放のフロー
- **VCイベント対応** — VC参加でのレンタル申請開始、10分タイムアウト通知
- **部屋主引き継ぎ** — 部屋主が退出した際に他の参加者へ引き継ぎ可能
- **多言語対応** — 日本語・英語に対応し、サーバーごとに設定可能

## クイックスタート

Bot の起動・ホスティング方法は [Bot運用者向け: セットアップ](docs/ja/Bot運用者向け/セットアップ.md) を参照してください。

## ドキュメント

| 対象 | リンク |
|---|---|
| Bot運用者 | [docs/ja/Bot運用者向け/](docs/ja/Bot運用者向け/) |
| サーバー管理者 | [docs/ja/サーバー管理者向け/](docs/ja/サーバー管理者向け/) |
| サーバー利用者 | [docs/ja/サーバー利用者向け/](docs/ja/サーバー利用者向け/) |
| 開発者 | [docs/ja/開発者向け/](docs/ja/開発者向け/) |

## Botコマンド（概要）

| コマンド | 権限 | 説明 |
|---|---|---|
| `/register_report_channel` | 管理者 | 通報先チャンネルを登録 |
| `/register_rental_button_channel` | 管理者 | レンタルボタン用チャンネルを登録 |
| `/register_room` | 管理者 | 使用可能な部屋を登録 |
| `/delete_room` | 管理者 | 登録された部屋を削除 |
| `/rent` | 全員 | レンタル申請を行う |
| `/help` | 全員 | ヘルプを表示 |

詳細は [コマンドリファレンス](docs/ja/サーバー管理者向け/コマンドリファレンス.md) を参照してください。

## 技術スタック

Rust · [Twilight](https://twilight.rs) · [SeaORM](https://www.sea-ql.org/SeaORM/) · PostgreSQL · Docker Compose

## ライセンス

MIT — [LICENSE](LICENSE) を参照してください。
