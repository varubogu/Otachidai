## レンタルフロー
bot-rental-request-start = 利用目的を入力してください。10分以内に送信してください。
bot-rental-purpose-label = 利用目的
bot-rental-answers-label = 質問への回答
bot-rental-answer-prefix = 回答
bot-rental-assigned = 部屋が割り当てられました！{ $channel } をご利用ください。
bot-rental-timeout = レンタルリクエストがタイムアウトしました。もう一度お試しください。
bot-rental-report = ユーザー { $user } がボイスチャンネルに参加しましたが、10分以内に利用目的を送信しませんでした。
bot-rental-released = 部屋を解放しました。ご利用ありがとうございました。
bot-rental-no-rooms = 現在利用可能な部屋がありません。しばらくしてからもう一度お試しください。
bot-rental-already-renting = すでにアクティブなレンタルがあります。
bot-rental-room-occupied = その部屋は現在使用中です。
bot-rental-dropdown-prompt = 以下の質問に回答し、「確認」ボタンを押してください。
bot-rental-dropdown-confirm = 確認
bot-rental-expired = レンタル申請の有効期限が切れました。もう一度申請してください。

## 引き継ぎ
bot-handoff-prompt = 部屋のホストが退出しました。引き継ぎますか？
bot-handoff-accepted = { $user } が新しいホストになりました。
bot-handoff-timeout = 誰も引き継ぎませんでした。部屋を解放しました。
bot-handoff-take-over = 引き継ぐ

## 管理者コマンド
admin-report-channel-registered = レポートチャンネルを登録しました: { $channel }
admin-rental-button-registered = レンタルボタンチャンネルを登録しました。{ $channel } にボタンを投稿しました。
admin-question-preset-saved = 質問プリセットを保存しました。
admin-question-preset-name-required = プリセット名を指定してください。
admin-question-preset-at-least-one = question_1 ～ question_10 のいずれかを指定してください。
admin-question-preset-not-found = 指定された質問プリセットが見つかりませんでした。
admin-room-registered = 部屋を登録しました。
admin-room-deleted = 部屋を削除しました。
admin-room-not-found = 指定されたチャンネルIDの部屋が見つかりませんでした。
admin-permission-denied = このコマンドを使用するにはサーバー管理者権限が必要です。
admin-room-at-least-one = text_channel_id または voice_channel_id のいずれかが必要です。

## ヘルプ
help-title = otachidai Bot — ヘルプ
help-user = **ユーザーコマンド**
    `/rent` — レンタルリクエストを開始する
    `/help` — このヘルプを表示する
help-admin = **管理者コマンド**
    `/register_report_channel` — タイムアウト通知チャンネルを登録する
    `/register_rental_button_channel` — レンタルボタンを投稿するチャンネルを登録する
    `/register_question_preset` — 部屋ごとの質問プリセットを登録する
    `/register_room` — 部屋を登録する（テキストチャンネル、ボイスチャンネル、またはその両方）
    `/delete_room` — 登録済み部屋を削除する

## レンタルボタン
rent-button-label = 部屋をリクエスト

## エラー
error-generic = エラーが発生しました。もう一度お試しください。
error-db = データベースエラーが発生しました。ボット運営者にお問い合わせください。
