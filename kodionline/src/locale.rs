use fluent_templates;
use fluent_templates::Loader;
use unic_langid::{langid, LanguageIdentifier};

fluent_templates::static_loader! {
    pub static LOCALES = {
        locales: "../locales",
        fallback_language: "en-US",
        customise: |bundle| bundle.set_use_isolating(false),
    };
}

pub fn get_ui_locale() -> LanguageIdentifier {
    langid!("en")
}
