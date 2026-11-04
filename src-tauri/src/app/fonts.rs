//! The installed monospace fonts, for the Settings window's code font
//! (PRD §6.8).

use std::sync::OnceLock;

/// The family names of the installed monospace fonts, sorted. Fonts are read
/// once, the first time they're asked for.
pub fn monospace_fonts() -> Vec<String> {
    static FONTS: OnceLock<Vec<String>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut db = fontdb::Database::new();
            db.load_system_fonts();
            monospace_families(db.faces().map(|face| {
                (
                    face.families.first().map(|(name, _)| name.as_str()),
                    face.monospaced,
                )
            }))
        })
        .clone()
}

/// The names of the monospaced faces' families, without duplicates (a family
/// has a face per weight and style), sorted ignoring case.
fn monospace_families<'a>(faces: impl Iterator<Item = (Option<&'a str>, bool)>) -> Vec<String> {
    let mut names: Vec<String> = faces
        .filter_map(|(name, monospaced)| name.filter(|_| monospaced))
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect();
    names.sort_by_cached_key(|name| (name.to_lowercase(), name.clone()));
    names.dedup();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_each_monospace_family_once_in_order() {
        let faces = [
            (Some("JetBrains Mono"), true),
            (Some("DejaVu Sans"), false),
            (Some("dejavu sans mono"), true),
            (Some("JetBrains Mono"), true),
            (None, true),
            (Some("  "), true),
            (Some("Fira Code"), true),
        ];
        assert_eq!(
            monospace_families(faces.into_iter()),
            ["dejavu sans mono", "Fira Code", "JetBrains Mono"]
        );
    }
}
