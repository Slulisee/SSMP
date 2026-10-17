use std::{collections::HashMap, sync::OnceLock};
static STRINGS: OnceLock<HashMap<String, HashMap<String, String>>> = OnceLock::new();
pub fn tr(language: &str, key: &'static str) -> &'static str {
    STRINGS
        .get_or_init(|| {
            serde_json::from_str(include_str!("../../assets/languages.json"))
                .expect("bundled language catalog")
        })
        .get(language)
        .and_then(|m| m.get(key))
        .map(String::as_str)
        .unwrap_or(key)
}
pub fn translate(language: &str, text: &str) -> String {
    let catalog = STRINGS.get_or_init(|| {
        serde_json::from_str(include_str!("../../assets/languages.json"))
            .expect("bundled language catalog")
    });
    catalog
        .get(language)
        .and_then(|m| m.get(text))
        .cloned()
        .unwrap_or_else(|| text.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_bundled_catalogs() {
        let value: serde_json::Value =
            serde_json::from_str(include_str!("../../assets/languages.json")).unwrap();
        let english = value["en"].as_object().unwrap();
        assert!(english.len() > 190);
        for lang in ["de", "es", "fr"] {
            let catalog = value[lang].as_object().unwrap();
            for key in english.keys() {
                assert!(
                    catalog[key].as_str().is_some_and(|s| !s.is_empty()),
                    "{lang}: {key}"
                );
            }
        }
        assert_eq!(tr("de", "Artists"), "Künstler");
        assert_eq!(tr("es", "Lyrics"), "Letra");
        assert_eq!(tr("fr", "Settings"), "Paramètres");
    }
}
