新しいi18nメッセージキーを追加します。引数としてキー名（例: `bot-rental-foo`）とメッセージ内容（日本語・英語）を受け取ります。

以下の3ファイルをすべて更新してください:

1. **`locales/en/main.ftl`** — 英語メッセージを追加（既存の同カテゴリのブロック末尾に）
2. **`locales/ja/main.ftl`** — 日本語メッセージを追加（同位置）
3. **`src/i18n/messages.rs`** — `MessageKey` enumに新しいバリアントを追加し、`as_str()` の `match` アームにも対応するFTLキー文字列を追加

### 命名規則

- FTLキー: `kebab-case`（例: `bot-rental-foo`）
- Rustバリアント: `PascalCase`で同じ語を連結（例: `BotRentalFoo`）

### プレースホルダ付きメッセージの例

```
# locales/en/main.ftl
bot-rental-assigned = Room assigned! Please use { $channel }.

# locales/ja/main.ftl
bot-rental-assigned = 部屋が割り当てられました！{ $channel } をご利用ください。
```

呼び出し側では `i18n.get_with_args(lang, &MessageKey::BotRentalAssigned, Some(&args))` を使います。

$ARGUMENTS
