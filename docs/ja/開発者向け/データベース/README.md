# データベース設計

お立ち台BotのPostgreSQLスキーマと、その上に乗っているアクセス制御の設計書です。
コードの実装は `src/entities/`（ORMエンティティ）、`src/facade/`（ドメイン別クエリ）、`migration/src/`（マイグレーション）に分散しているため、ここでは**スキーマ構造**と**ロール／行レベル制御の設計**を中心にまとめます。

## このフォルダのファイル

| ファイル | 内容 |
|---|---|
| [ER図.md](ER図.md) | テーブル間のリレーションを Mermaid で可視化 |
| [テーブル一覧.md](テーブル一覧.md) | 全テーブルのカラム仕様・制約・コード値マッピング |
| [ロールとRLS.md](ロールとRLS.md) | 5つのDBロール、RLSポリシー、ガード機構（`with_guild_context`） |

## スキーマ構成

PostgreSQL内に2つのアプリケーションスキーマを置きます。

| スキーマ | 用途 | 含まれるテーブル |
|---|---|---|
| `guild_master` | ギルド単位のマスターデータ・運用状態 | `guilds`、`guild_channels`、`rooms`、`rental_question_presets`、`rental_sessions` |
| `worker` | バックグラウンド処理（タイマー・通知）のためのワーキング領域 | `scheduled_tasks`、`notifications` |

スキーマを分けているのは、**「ドメインデータ」と「処理キュー」を境界として明確に分離する**ためです。RLSも `guild_master` のみに適用し、`worker` はスケジューラ／クリーンアップ系ロールが横断的に扱う前提になっています。

## ロールの一覧（概要）

詳細は [ロールとRLS.md](ロールとRLS.md) を参照。

| ロール | RLS | 主用途 |
|---|---|---|
| `otachidai_bot_system` | BYPASS | スケジューラ／タイマー復元 |
| `otachidai_bot_guild` | 適用 | スラッシュコマンド・VCイベントなどギルド単位処理 |
| `otachidai_bot_global` | BYPASS | マスターデータ更新（外部同期等） |
| `otachidai_bot_admin` | BYPASS | マイグレーション、スキーマ変更 |
| `otachidai_bot_cleanup` | BYPASS | データ削除専用 |

## 設計原則

### スキーマ層でのギルド分離

複数サーバーが同居する1つのDB上で、あるサーバーのデータが別サーバーから見えないことを**アプリケーションのチェックではなくRLSで強制**します。
通常運用で使う `guild_master.*` の操作はすべて `db::rls::with_guild_context()` 経由で行い、トランザクション内で `SET LOCAL app.current_guild_id` を設定します。詳しくは [ロールとRLS.md](ロールとRLS.md) を参照。

### 状態の二重持ち（メモリとDB）

レンタルセッションの状態は `RentalStateMap`（`DashMap`）と `rental_sessions.state` の両方に持ちます。
メモリ側は応答性のため、DB側は再起動・障害時の復元のためです。`scheduled_tasks` も同じ理由で永続化されており、起動時に `restore_pending_timeouts` で未処理タスクを再 spawn します。

### 不変履歴ではなく可変ステータス

`rental_sessions` は1セッション1行で、状態遷移は `state` カラムを書き換えて行います（履歴テーブルは別途持たない）。
監査要件が出てきた場合は `rental_sessions.state` の更新ログを `worker` スキーマ等に分離して持つ拡張余地を残しています。

## 関連ドキュメント

- [基本設計.md](../基本設計.md) — Botのコンセプトと設計原則
- [アーキテクチャ.md](../アーキテクチャ.md) — モジュール構成・状態遷移
- [設定ファイル仕様.md](../設定ファイル仕様.md) — DBの環境変数とギルド単位の設定書き換え経路
