//! The script each window runs before its page loads (TECH.md §6).

use super::settings::Settings;

const SCRIPT: &str = include_str!("boot.js");

/// `boot.js` with the current settings filled in.
pub fn script(settings: &Settings) -> String {
    let settings = serde_json::to_string(settings).expect("settings serialise to JSON");
    SCRIPT.replace("__SETTINGS__", &settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::Appearance;

    #[test]
    fn fills_in_the_settings() {
        let settings = Settings {
            appearance: Appearance::Dark,
            ..Default::default()
        };
        let script = script(&settings);
        assert!(!script.contains("__SETTINGS__"));
        assert!(script.contains(r#"var settings = {"appearance":"dark","theme":"github","#));
    }
}
