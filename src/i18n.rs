//! Localisation via Fluent.
//!
//! The `.ftl` catalogues under `i18n/` are embedded in the binary.  The
//! language is negotiated from the environment (`LC_ALL`, `LC_MESSAGES`,
//! `LANG`), falling back to English.  Use the crate-level [`fl!`] macro to
//! fetch messages; message ids and arguments are checked at compile time.

use std::sync::LazyLock;

use i18n_embed::fluent::{FluentLanguageLoader, fluent_language_loader};
use i18n_embed::{DesktopLanguageRequester, LanguageLoader};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "i18n/"]
struct Localizations;

pub static LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| {
    let loader: FluentLanguageLoader = fluent_language_loader!();
    let requested = DesktopLanguageRequester::requested_languages();
    let selected = i18n_embed::select(&loader, &Localizations, &requested);
    if !selected.is_ok_and(|langs| !langs.is_empty()) {
        // The English catalogue is embedded, so this cannot fail.
        let _ = loader.load_fallback_language(&Localizations);
    }
    // Fluent wraps arguments in Unicode isolation marks by default; they are
    // useless in a terminal and confuse width calculations.
    loader.set_use_isolating(false);
    loader
});

/// Translate a message: `fl!("id")` or `fl!("id", arg = value, ...)`.
#[macro_export]
macro_rules! fl {
    ($message_id:literal) => {{
        i18n_embed_fl::fl!($crate::i18n::LOADER, $message_id)
    }};
    ($message_id:literal, $($args:expr),* $(,)?) => {{
        i18n_embed_fl::fl!($crate::i18n::LOADER, $message_id, $($args),*)
    }};
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn message_ids(ftl: &str) -> BTreeSet<&str> {
        ftl.lines()
            .filter(|l| l.starts_with(|c: char| c.is_ascii_lowercase()))
            .filter_map(|l| l.split_once(" =").map(|(id, _)| id))
            .collect()
    }

    #[test]
    fn catalogue_is_loaded_at_runtime() {
        assert!(LOADER.has("app-name"));
        assert!(!fl!("app-name").contains("No localization"));
    }

    #[test]
    fn translations_have_the_same_messages() {
        let en = message_ids(include_str!("../i18n/en/meshtastic_node_manager.ftl"));
        let es = message_ids(include_str!("../i18n/es/meshtastic_node_manager.ftl"));
        assert!(en.len() > 50);
        assert_eq!(en, es);
    }
}
