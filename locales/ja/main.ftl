## レンタルフロー
bot-rental-request-start = 利用目的を入力してください。10分以内に送信してください。
bot-rental-answer-prefix = 回答
bot-rental-assigned = 部屋が割り当てられました！{ $channel } をご利用ください。
bot-rental-timeout = レンタルリクエストがタイムアウトしました。もう一度お試しください。
bot-rental-report = ユーザー { $user } がボイスチャンネルに参加しましたが、10分以内に利用目的を送信しませんでした。
bot-rental-released = 部屋を解放しました。ご利用ありがとうございました。
bot-rental-no-rooms = 現在利用可能な部屋がありません。しばらくしてからもう一度お試しください。
bot-rental-already-renting = すでにアクティブなレンタルがあります。
bot-rental-room-occupied = その部屋は現在使用中です。
bot-rental-expired = レンタル申請の有効期限が切れました。もう一度申請してください。
bot-rental-vc-question = 利用するVCを選択してください
bot-rental-vc-room-occupied = 選択した部屋は他のレンタルで使用中です。別の部屋を選択してください。
bot-rental-vc-no-rooms = 選択可能なVCがありません。

## 引き継ぎ
bot-handoff-prompt = 部屋のホストが退出しました。引き継ぎますか？
bot-handoff-accepted = { $user } が新しいホストになりました。
bot-handoff-timeout = 誰も引き継ぎませんでした。部屋を解放しました。
bot-handoff-take-over = 引き継ぐ

## レンタル状況ボード
status-title = レンタル状況
status-available = 空き
status-awaiting = 受付中
status-in-use = 利用中
status-pending-handoff = 引き継ぎ待ち
status-summary = 空き { $free } / 利用中 { $used }
status-no-rooms = まだ部屋が登録されていません。

## 管理者コマンド
admin-report-channel-registered = レポートチャンネルを登録しました: { $channel }
admin-rental-button-registered = レンタルボタンチャンネルを登録しました。{ $channel } にボタンを投稿しました。
admin-room-list-channel-registered = 部屋一覧チャンネルを登録しました。{ $channel } に部屋一覧を投稿します。
admin-question-preset-saved = 質問プリセットを保存しました。
admin-question-preset-name-required = プリセット名を指定してください。
admin-question-preset-at-least-one = question_1 ～ question_10 のいずれかを指定してください。
admin-question-preset-not-found = 指定された質問プリセットが見つかりませんでした。
admin-question-preset-deleted = 質問プリセットを削除しました。
admin-question-preset-list-empty = 登録済みの質問プリセットはありません。
admin-question-preset-list-header = 登録済みの質問プリセット:
admin-room-registered = 部屋を登録しました。
admin-room-deleted = 部屋を削除しました。
admin-room-not-found = 指定されたチャンネルIDの部屋が見つかりませんでした。
admin-permission-denied = このコマンドを使用するにはサーバー管理者権限が必要です。
admin-room-at-least-one = text_channel_id または voice_channel_id のいずれかが必要です。
admin-group-registered = グループ「{ $name }」を登録しました。ステータスボードは { $channel } に投稿されます。
admin-group-deleted = グループ「{ $name }」を削除しました。
admin-group-not-found = グループ「{ $name }」が見つかりません。
admin-group-exists = グループ「{ $name }」は既に存在します。
admin-group-name-required = グループ名を指定してください。
admin-room-group-updated = 部屋をグループ「{ $name }」に移動しました。
admin-room-group-cleared = 部屋のグループ所属を解除しました。
admin-room-preset-updated = 部屋の質問プリセットを「{ $name }」に変更しました。
admin-room-preset-cleared = 部屋の質問プリセットを解除しました。
admin-room-list-empty = 登録済みの部屋はありません。
admin-room-list-header = 登録済みの部屋:
admin-room-list-item = [{ $id }] { $channels } | プリセット: { $preset } | グループ: { $group }
admin-room-list-none = （なし）

## ヘルプ
help-title = otachidai Bot — ヘルプ
help-user = **ユーザーコマンド**
    `/rent` — レンタルリクエストを開始する
    `/help` — このヘルプを表示する
help-admin = **管理者コマンド**
    `/upload_guild_config` — ギルド設定 (チャンネル / 質問プリセット / 部屋 / ルーティング) を YAML で一括アップロード
    `/download_guild_config` — 現在のギルド設定を YAML としてダウンロード
    `/list_question_presets` — 登録済みの質問プリセットを一覧表示する
    `/list_rooms` — 登録済み部屋を一覧表示する

## レンタルボタン
rent-button-label = 部屋をリクエスト
rent-answer-button-label = 回答する

## エラー
error-generic = エラーが発生しました。もう一度お試しください。
error-db = データベースエラーが発生しました。ボット運営者にお問い合わせください。

## ギルド設定 (YAML)
bot-config-upload-success = ギルド設定を更新しました。
bot-config-upload-active-sessions-released = ギルド設定を更新しました。利用中だった { $count } 件のレンタルセッションは強制的に終了しました。
bot-config-upload-error-yaml = YAML の解析に失敗しました:
    { $detail }
bot-config-upload-error-validation = ギルド設定にエラーがあります:
    { $detail }
bot-config-upload-error-attachment = YAML ファイルを取得できませんでした。ファイルサイズや拡張子を確認してください。
bot-config-download-empty = 現在登録されているギルド設定はありません。

## 利用目的の自動投稿
bot-rental-post-default-template = { $user } さんが { $room } の利用を開始しました
    { $answers }
bot-rental-force-released = ギルド設定の更新により、現在のレンタルは強制的に終了しました。再度 `/rent` でリクエストしてください。
