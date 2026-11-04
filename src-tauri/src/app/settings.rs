//! Application settings (PRD §6.8): defaults, merging a patch, and reading
//! what's stored leniently.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use turbodiff_repo::Algorithm;

pub const FONT_SIZES: std::ops::RangeInclusive<u8> = 10..=24;
pub const TAB_WIDTHS: [u8; 3] = [2, 4, 8];
pub const CONTEXT_LINES: std::ops::RangeInclusive<u8> = 0..=20;
pub const SIDEBAR_WIDTHS: std::ops::RangeInclusive<u16> = 160..=800;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Appearance {
    /// Follow the OS light or dark preference.
    #[default]
    System,
    Light,
    Dark,
}

/// Only GitHub ships in v1 (PRD §4).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    #[default]
    Github,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum DiffStyle {
    #[default]
    Inline,
    Split,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub appearance: Appearance,
    pub theme: Theme,
    pub diff_style: DiffStyle,
    /// A font family name. Empty means the platform's default monospace font.
    pub code_font: String,
    pub font_size: u8,
    pub tab_width: u8,
    pub line_wrapping: bool,
    pub context_lines: u8,
    pub diff_algorithm: Algorithm,
    /// The sidebar width in pixels. Not shown in the Settings window, but
    /// kept with the settings (TECH.md §4.7).
    pub sidebar_width: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            appearance: Appearance::System,
            theme: Theme::Github,
            diff_style: DiffStyle::Inline,
            code_font: String::new(),
            font_size: 13,
            tab_width: 4,
            line_wrapping: false,
            context_lines: 3,
            diff_algorithm: Algorithm::Histogram,
            sidebar_width: 280,
        }
    }
}

/// The settings to change. Missing fields are left as they are.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsPatch {
    #[specta(optional)]
    pub appearance: Option<Appearance>,
    #[specta(optional)]
    pub theme: Option<Theme>,
    #[specta(optional)]
    pub diff_style: Option<DiffStyle>,
    #[specta(optional)]
    pub code_font: Option<String>,
    #[specta(optional)]
    pub font_size: Option<u8>,
    #[specta(optional)]
    pub tab_width: Option<u8>,
    #[specta(optional)]
    pub line_wrapping: Option<bool>,
    #[specta(optional)]
    pub context_lines: Option<u8>,
    #[specta(optional)]
    pub diff_algorithm: Option<Algorithm>,
    #[specta(optional)]
    pub sidebar_width: Option<u16>,
}

impl Settings {
    /// Applies `patch`. Numbers are clamped to their range, and a tab width
    /// that isn't offered is ignored.
    pub fn apply(&mut self, patch: SettingsPatch) {
        let SettingsPatch {
            appearance,
            theme,
            diff_style,
            code_font,
            font_size,
            tab_width,
            line_wrapping,
            context_lines,
            diff_algorithm,
            sidebar_width,
        } = patch;
        set(&mut self.appearance, appearance);
        set(&mut self.theme, theme);
        set(&mut self.diff_style, diff_style);
        set(
            &mut self.code_font,
            code_font.map(|font| font.trim().to_owned()),
        );
        set(
            &mut self.font_size,
            font_size.map(|size| clamp(size, &FONT_SIZES)),
        );
        set(
            &mut self.tab_width,
            tab_width.filter(|width| TAB_WIDTHS.contains(width)),
        );
        set(&mut self.line_wrapping, line_wrapping);
        set(
            &mut self.context_lines,
            context_lines.map(|lines| clamp(lines, &CONTEXT_LINES)),
        );
        set(&mut self.diff_algorithm, diff_algorithm);
        set(
            &mut self.sidebar_width,
            sidebar_width.map(|width| clamp(width, &SIDEBAR_WIDTHS)),
        );
    }

    /// Restores every setting shown in the Settings window. The sidebar width
    /// isn't one of them, so it stays.
    pub fn reset(&mut self) {
        *self = Self {
            sidebar_width: self.sidebar_width,
            ..Self::default()
        };
    }

    /// Reads stored settings. Anything missing or invalid gets its default,
    /// field by field, so one bad value doesn't reset the rest.
    pub fn from_stored(stored: Option<&Value>) -> Self {
        let mut settings = Self::default();
        if let Some(Value::Object(fields)) = stored {
            for (key, value) in fields {
                let field = Value::Object([(key.clone(), value.clone())].into_iter().collect());
                if let Ok(patch) = serde_json::from_value::<SettingsPatch>(field) {
                    settings.apply(patch);
                }
            }
        }
        settings
    }
}

fn set<T>(field: &mut T, value: Option<T>) {
    if let Some(value) = value {
        *field = value;
    }
}

fn clamp<T: Ord + Copy>(value: T, range: &std::ops::RangeInclusive<T>) -> T {
    value.clamp(*range.start(), *range.end())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn defaults_match_the_prd() {
        let settings = Settings::default();
        assert_eq!(settings.appearance, Appearance::System);
        assert_eq!(settings.diff_style, DiffStyle::Inline);
        assert_eq!(settings.font_size, 13);
        assert_eq!(settings.tab_width, 4);
        assert!(!settings.line_wrapping);
        assert_eq!(settings.context_lines, 3);
        assert_eq!(settings.diff_algorithm, Algorithm::Histogram);
    }

    #[test]
    fn a_patch_changes_only_its_fields() {
        let mut settings = Settings::default();
        settings.apply(SettingsPatch {
            diff_style: Some(DiffStyle::Split),
            code_font: Some("  Fira Code ".into()),
            ..Default::default()
        });
        assert_eq!(
            settings,
            Settings {
                diff_style: DiffStyle::Split,
                code_font: "Fira Code".into(),
                ..Default::default()
            }
        );
    }

    #[test]
    fn numbers_are_clamped_and_bad_tab_widths_ignored() {
        let mut settings = Settings::default();
        settings.apply(SettingsPatch {
            font_size: Some(99),
            context_lines: Some(50),
            tab_width: Some(3),
            sidebar_width: Some(10),
            ..Default::default()
        });
        assert_eq!(settings.font_size, 24);
        assert_eq!(settings.context_lines, 20);
        assert_eq!(settings.tab_width, 4);
        assert_eq!(settings.sidebar_width, 160);
        settings.apply(SettingsPatch {
            font_size: Some(1),
            tab_width: Some(8),
            ..Default::default()
        });
        assert_eq!(settings.font_size, 10);
        assert_eq!(settings.tab_width, 8);
    }

    #[test]
    fn reset_restores_defaults_but_keeps_the_sidebar_width() {
        let mut settings = Settings {
            appearance: Appearance::Dark,
            code_font: "Fira Code".into(),
            font_size: 18,
            context_lines: 10,
            diff_algorithm: Algorithm::Myers,
            sidebar_width: 400,
            ..Default::default()
        };
        settings.reset();
        assert_eq!(
            settings,
            Settings {
                sidebar_width: 400,
                ..Default::default()
            }
        );
    }

    #[test]
    fn a_patch_deserialises_from_partial_json() {
        let patch: SettingsPatch =
            serde_json::from_value(json!({ "appearance": "dark", "diffAlgorithm": "myers" }))
                .unwrap();
        assert_eq!(
            patch,
            SettingsPatch {
                appearance: Some(Appearance::Dark),
                diff_algorithm: Some(Algorithm::Myers),
                ..Default::default()
            }
        );
    }

    #[test]
    fn stored_settings_round_trip() {
        let settings = Settings {
            appearance: Appearance::Light,
            line_wrapping: true,
            sidebar_width: 400,
            ..Default::default()
        };
        let stored = serde_json::to_value(&settings).unwrap();
        assert_eq!(Settings::from_stored(Some(&stored)), settings);
    }

    #[test]
    fn bad_stored_values_fall_back_one_by_one() {
        let stored = json!({
            "appearance": "dark",
            "fontSize": "huge",
            "theme": "solarized",
            "tabWidth": 8,
            "unknown": true,
        });
        let settings = Settings::from_stored(Some(&stored));
        assert_eq!(
            settings,
            Settings {
                appearance: Appearance::Dark,
                tab_width: 8,
                ..Default::default()
            }
        );
        assert_eq!(Settings::from_stored(None), Settings::default());
        assert_eq!(
            Settings::from_stored(Some(&json!("garbage"))),
            Settings::default()
        );
    }
}
