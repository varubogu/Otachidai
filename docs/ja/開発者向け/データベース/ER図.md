# ER図

テーブル間のリレーションを示します。Discord ID（`guild_id`、`*_channel_id`、`host_user_id`、`task_id`）はBIGINTで、Discord 側のスノーフレークIDをそのまま格納します。

## 全体図

```mermaid
erDiagram
    guilds ||--o{ guild_channels : has
    guilds ||--o{ rooms : has
    guilds ||--o{ rental_question_presets : has
    guilds ||--o{ rental_sessions : has
    rental_question_presets ||--o{ rooms : "preset (SET NULL)"
    rooms ||--o{ rental_sessions : "rented as"
    rental_sessions ||--o{ scheduled_tasks : "schedules"
    scheduled_tasks ||--o{ notifications : "fires"

    guilds {
        BIGINT      guild_id PK "Discord guild id"
        VARCHAR(5)  language  "ja / en (default ja)"
        TIMESTAMPTZ created_at
        TIMESTAMPTZ updated_at
    }

    guild_channels {
        SERIAL      id PK
        BIGINT      guild_id FK
        BIGINT      channel_id "Discord channel id"
        SMALLINT    channel_type "1=report, 2=rental_button"
        TIMESTAMPTZ created_at
    }

    rooms {
        SERIAL      id PK
        BIGINT      guild_id FK
        BIGINT      text_channel_id "nullable"
        BIGINT      voice_channel_id "nullable"
        BOOLEAN     is_available "default TRUE"
        INT         question_preset_id FK "nullable"
        TIMESTAMPTZ created_at
    }

    rental_question_presets {
        SERIAL      id PK
        BIGINT      guild_id FK
        TEXT        name "unique within guild"
        TEXT        question_1 "nullable"
        TEXT        question_2 "nullable"
        TEXT        question_n "...up to question_10"
        TIMESTAMPTZ created_at
        TIMESTAMPTZ updated_at
    }

    rental_sessions {
        SERIAL      id PK
        BIGINT      guild_id FK
        INT         room_id FK
        BIGINT      host_user_id "Discord user id"
        TEXT        purpose "nullable"
        SMALLINT    state "1=awaiting,2=active,3=released,4=pending_handoff"
        TIMESTAMPTZ started_at
        TIMESTAMPTZ purpose_deadline "nullable"
        TIMESTAMPTZ ended_at "nullable"
    }

    scheduled_tasks {
        SERIAL      id PK
        BIGINT      guild_id "denormalized"
        SMALLINT    task_type "1=timeout_notification"
        INT         rental_session_id FK "nullable"
        TIMESTAMPTZ schedule_datetime
        BOOLEAN     processed "default FALSE"
        TIMESTAMPTZ created_at
    }

    notifications {
        SERIAL      id PK
        INT         task_id FK
        BIGINT      guild_id "denormalized"
        TIMESTAMPTZ schedule_datetime
        BOOLEAN     sent "default FALSE"
        TIMESTAMPTZ sent_at "nullable"
        TIMESTAMPTZ created_at
    }
```

## 関係の読み解き

### ギルドツリー（`guild_master` スキーマ）

`guilds` を頂点に、子テーブルがすべて `guild_id` を持ちます。
`ON DELETE CASCADE` を `guild_channels`／`rooms`／`rental_question_presets` に張っているため、ギルドが削除されればこれらは芋づる式に消えます。`rental_sessions` だけは `guild_id` への ON DELETE 指定がなく（NO ACTION）、ギルド削除前に履歴を別の方法で扱う想定です。

`rooms` → `rental_question_presets` は `ON DELETE SET NULL`：プリセットを消しても部屋は残り、紐付け情報だけ外れます。

### セッションとスケジューラ（`worker` スキーマへの橋渡し）

`scheduled_tasks` は `rental_sessions(id)` に `ON DELETE CASCADE` で参照しています。セッションを物理削除するとスケジュールも自動消去されますが、運用上はセッションを物理削除せず `state=3 (released)` に更新する設計です。

`scheduled_tasks` と `notifications` は `guild_id` を**非正規化**で持ちます。`worker` スキーマには RLS が掛かっていないため、 `guild_id` は分離のための論理キー兼インデックス用途です。

### 一意性制約

| テーブル | UNIQUE 制約 | 意味 |
|---|---|---|
| `guild_channels` | `(guild_id, channel_type)` | 1ギルドにつき1種別1チャンネル |
| `rooms` | `(guild_id, text_channel_id)` | 同じテキストCHを2部屋に登録できない |
| `rooms` | `(guild_id, voice_channel_id)` | 同じVCを2部屋に登録できない |
| `rental_question_presets` | `(guild_id, name)` | プリセット名はギルド内一意 |

### 任意フィールドの組み合わせ

`rooms` は `text_channel_id` と `voice_channel_id` のいずれもNULL可です。「テキストのみ」「VCのみ」「セット」の3形態を1つのテーブルで表現するため、**少なくとも一方が値を持つ**ことはアプリケーション側のバリデーション（`/register_room` の `AdminRoomAtLeastOne`）で担保しています。
