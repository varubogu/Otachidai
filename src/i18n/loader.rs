use crate::error::{BotError, BotResult};
use crate::facade::guild_settings::DEFAULT_LANGUAGE;
use crate::i18n::MessageKey;
use fluent_bundle::{FluentArgs, FluentResource, concurrent::FluentBundle as ConcurrentBundle};
use std::collections::HashMap;
use unic_langid::LanguageIdentifier;

pub struct I18n {
    bundles: HashMap<String, ConcurrentBundle<FluentResource>>,
    default_lang: String,
}

impl I18n {
    pub fn load(locales_dir: &str) -> BotResult<Self> {
        let mut bundles = HashMap::new();
        let default_lang = DEFAULT_LANGUAGE.to_string();

        for lang in &["en", "ja"] {
            let path = format!("{locales_dir}/{lang}/main.ftl");
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            if content.is_empty() {
                tracing::warn!("Locale file not found or empty: {path}");
                continue;
            }
            let bundle = build_bundle(lang, content)?;
            bundles.insert(lang.to_string(), bundle);
        }

        if bundles.is_empty() {
            return Err(BotError::I18n(
                "No locale bundles could be loaded".to_string(),
            ));
        }

        Ok(I18n {
            bundles,
            default_lang,
        })
    }

    pub fn get(&self, lang: &str, key: &MessageKey) -> String {
        self.get_with_args(lang, key, None)
    }

    pub fn get_with_args(
        &self,
        lang: &str,
        key: &MessageKey,
        args: Option<&FluentArgs<'_>>,
    ) -> String {
        let bundle = self
            .bundles
            .get(lang)
            .or_else(|| self.bundles.get(&self.default_lang))
            .expect("default locale bundle must exist");

        let msg_key = key.as_str();
        let msg = match bundle.get_message(msg_key) {
            Some(m) => m,
            None => {
                tracing::warn!("Missing i18n key: {msg_key}");
                return msg_key.to_string();
            }
        };

        let pattern = match msg.value() {
            Some(p) => p,
            None => {
                tracing::warn!("i18n key has no value: {msg_key}");
                return msg_key.to_string();
            }
        };

        let mut errors = vec![];
        let value = bundle.format_pattern(pattern, args, &mut errors);
        if !errors.is_empty() {
            tracing::warn!("i18n format errors for {msg_key}: {errors:?}");
        }
        value.into_owned()
    }
}

fn build_bundle(lang: &str, content: String) -> BotResult<ConcurrentBundle<FluentResource>> {
    let langid: LanguageIdentifier = lang
        .parse()
        .map_err(|_| BotError::I18n(format!("Invalid language identifier: {lang}")))?;
    let res = FluentResource::try_new(content)
        .map_err(|(_, errs)| BotError::I18n(format!("FTL parse errors: {errs:?}")))?;
    let mut bundle = ConcurrentBundle::new_concurrent(vec![langid]);
    bundle
        .add_resource(res)
        .map_err(|errs| BotError::I18n(format!("FTL resource errors: {errs:?}")))?;
    Ok(bundle)
}
