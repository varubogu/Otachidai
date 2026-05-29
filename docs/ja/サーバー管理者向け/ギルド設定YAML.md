# ギルド設定 YAML 仕様

`/upload_guild_config` および `/download_guild_config` で扱うファイル形式の仕様です。

YAML ファイルはあくまで管理者と Bot 間のやり取り用フォーマットです。実体は DB に保存され、ファイルはアップロード処理後に破棄されます。

## ファイル全体構造

```yaml
version: 1

guild:
  language: ja                       # "ja" または "en"

channels:
  report: "111111111111111111"           # タイムアウト通知
  rental_button: "222222222222222222"    # レンタルボタンを設置するch
  room_list: "333333333333333333"        # 部屋一覧（ステータスボード）を投稿するch
  rental_post_fallback:                  # ルーティング不一致時の投稿先
    channel: "444444444444444444"
    template: |
      {{user}} さんが {{room}} の利用を開始しました
      {{answers}}

question_presets:
  - name: "通常募集"
    questions:
      - text: "目的は？"
        answers: ["雑談", "作業", "イベント"]   # 省略時は自由テキスト入力
        routing_key: true                       # プリセット内で最大 1 つだけ
      - text: "募集人数"
        answers: ["1", "2", "3", "4"]
      - text: "備考"
        # answers 省略 → 自由テキスト入力欄

room_groups:
  - name: "メインフロア"
    channel_id: "555555555555555555"           # このグループのステータスボード投稿先

rooms:
  - voice_channel_id: "666666666666666666"     # 部屋の同定子。重複不可
    text_channel_id: "777777777777777777"      # 任意
    group: "メインフロア"                       # room_groups[].name
    question_preset: "通常募集"                 # question_presets[].name

routing_rules:
  - preset: "通常募集"
    rules:
      - when: "雑談"                            # routing_key 質問の回答
        channel: "888888888888888888"
        template: |
          🎙 雑談部屋オープン！ host: {{user}} / room: {{room}}
          人数: {{answer:募集人数}}
          備考: {{answer:備考}}
      - when: "作業"
        channel: "999999999999999999"
        # template 省略 → 組み込みデフォルトを適用
```

## バリデーション

- `version` は **1 固定**。他の値はエラー。
- すべてのチャンネル ID / VC ID は 17〜20 桁の数値文字列。
- 同一 YAML 内で名前 (`question_presets[].name`、`room_groups[].name`) は重複不可。`rooms` は `voice_channel_id` の重複不可。
- 参照 (`rooms[].group`、`rooms[].question_preset`、`routing_rules[].preset`) は同一 YAML 内に存在しなければエラー。
- 各プリセット内で `routing_key: true` は最大 1 つ。ルーティングルールがあるのにルーティングキー指定がなければエラー。
- ルーティングキーがドロップダウンの質問の場合、`routing_rules[].rules[].when` はその選択肢に含まれていることが必須（タイポ防止）。
- 1 プリセットあたり質問は最大 10 個。

## 適用セマンティクス（完全置換）

- YAML が **ギルド設定全体のソース・オブ・トゥルース** として扱われます。YAML に記載されないエンティティは削除されます。
- 削除対象に **アクティブなレンタルセッションを持つ部屋** が含まれる場合、そのセッションは強制的に終了します。ホストには通知が送られ、in-memory 状態とタイムアウトタスクも片付けられます。
- 全処理は単一の DB トランザクションで実行されます。途中でエラーが起きた場合は完全にロールバックされ、DB の状態は変わりません。

## テンプレート構文

ルーティング投稿のテンプレートは Mustache 風の `{{ name }}` 記法を使います。

- リテラル `{{` / `}}` を出力したい場合は `\{{` / `\}}` でエスケープ。
- 利用可能な変数：

| 変数 | 内容 |
|---|---|
| `{{user}}` | レンタル開始ユーザーのメンション (`<@...>`) |
| `{{room}}` | 部屋のテキストチャンネル mention（無ければ VC mention） |
| `{{when}}` | マッチしたルーティングキー回答（フォールバック時は空文字） |
| `{{preset}}` | 質問プリセット名 |
| `{{q1}}` 〜 `{{q10}}` | 各質問の回答（番号は YAML 上の questions 配列順） |
| `{{answer:質問名}}` | 質問名で参照する回答 |
| `{{answers}}` | 全質問・全回答を組み立てた既定の purpose 文字列（複数行） |

未知の変数や、プリセットに存在しない質問の参照はアップロード時のバリデーションで弾かれます。

テンプレートを省略した場合は組み込みデフォルト（i18n キー `bot-rental-post-default-template`）が適用されます。

## 運用フロー

1. `/download_guild_config` で現状を取得
2. ローカルで編集
3. `/upload_guild_config` で投入
4. 必要に応じて `/list_rooms` などで結果を確認
