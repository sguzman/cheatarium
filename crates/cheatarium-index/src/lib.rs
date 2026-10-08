//! Loss-tolerant indexing of original Libretro .cht records.
//!
//! This code does NOT decode a Game Genie/Action Replay code into executable
//! emulator memory writes. Exact original bytes remain in archive/.
//! Filename-based game associations are suggestions, never ROM verification.
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Default)]
struct PartialCheat {
    description: Option<String>,
    code: Option<String>,
    source_enabled: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Code {
    pub ordinal: usize,
    pub description: Option<String>,
    pub code: Option<String>,
    pub source_enabled: bool,
    pub verification: &'static str,
    pub role: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParsedCheats {
    pub declared_count: Option<usize>,
    pub codes: Vec<Code>,
    pub warnings: Vec<String>,
}

fn unquote(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        // Preserve interior escapes and literal characters. A code is data,
        // not an expression or a string to be evaluated.
        return value[1..value.len() - 1].to_owned();
    }
    value.to_owned()
}

pub fn parse_cht(text: &str) -> ParsedCheats {
    let mut declared_count = None;
    let mut cheats: BTreeMap<usize, PartialCheat> = BTreeMap::new();

    for line in text.lines() {
        let Some((key, value)) = line.trim().split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = unquote(value);
        if key == "cheats" {
            declared_count = value.trim().parse::<usize>().ok();
            continue;
        }
        let Some(suffix) = key.strip_prefix("cheat") else {
            continue;
        };
        let Some((ordinal, field)) = suffix.split_once('_') else {
            continue;
        };
        let Ok(index) = ordinal.parse::<usize>() else {
            continue;
        };
        let entry = cheats.entry(index).or_default();
        match field {
            "desc" => entry.description = Some(value),
            "code" => entry.code = Some(value),
            "enable" => entry.source_enabled = matches!(value.trim(), "true" | "1"),
            _ => {}
        }
    }

    let codes: Vec<_> = cheats
        .into_iter()
        .map(|(ordinal, part)| Code {
            ordinal,
            description: part.description,
            code: part.code,
            source_enabled: part.source_enabled,
            verification: "unverified",
            role: if part.code.is_some() { "code" } else { "section-heading" },
        })
        .collect();
    let mut warnings = Vec::new();
    let count = codes.len();
    if declared_count.is_some_and(|n| n != count) {
        warnings.push(format!(
            "declared {0} entries, found {count} indexed entries",
            declared_count.unwrap_or_default()
        ));
    }
    // Description-only indexed entries are intentional headings in many
    // Libretro NDS cheat files; they are not malformed executable cheats.
    ParsedCheats { declared_count, codes, warnings }
}

/// Parenthetical suffixes stripped here are only common *filename metadata*.
/// The full source filename and raw upstream path are always exported too.
fn is_metadata_suffix(inner: &str) -> bool {
    let lower = inner.trim().to_ascii_lowercase();
    const EXACT: &[&str] = &[
        "usa", "japan", "europe", "world", "asia", "korea", "china",
        "france", "germany", "spain", "italy", "brazil", "australia",
        "u", "e", "j", "game genie", "action replay", "pro action replay",
        "gameshark", "game shark", "code breaker", "codebreaker",
        "rumbles", "diff", "unl", "beta", "proto", "prototype",
        "virtual console", "sgb enhanced", "gbc", "gba",
    ];
    EXACT.contains(&lower.as_str())
        || lower.starts_with("rev ")
        || lower.starts_with("revision ")
        || lower.contains("game genie")
        || lower.contains("action replay")
        || lower.contains("gameshark")
        || lower.contains("code breaker")
        || lower.starts_with("usa,")
        || lower.starts_with("japan,")
        || lower.starts_with("europe,")
        || lower.starts_with("world,")
        || lower.starts_with("en,")
}

pub fn title_hint(stem: &str) -> String {
    let mut title = stem.trim();
    while let Some(prefix) = title.strip_suffix(')') {
        let Some((start, tag)) = prefix.rsplit_once('(') else {
            break;
        };
        if !is_metadata_suffix(tag) {
            break;
        }
        title = start.trim_end();
    }
    title.to_owned()
}

/// Advisory slug only. Identical slugs never imply identical ROM editions.
pub fn candidate_game_key(title: &str) -> String {
    let mut slug = String::new();
    let mut gap = false;
    for ch in title.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            if gap && !slug.is_empty() {
                slug.push('-');
            }
            slug.push(ch);
            gap = false;
        } else {
            gap = true;
        }
    }
    slug
}

pub fn format_hint(filename: &str) -> Option<&'static str> {
    let name = filename.to_ascii_lowercase();
    if name.contains("(game genie)") {
        Some("game-genie")
    } else if name.contains("(action replay)") || name.contains("(pro action replay)") {
        Some("action-replay")
    } else if name.contains("(gameshark)") || name.contains("(game shark)") {
        Some("gameshark")
    } else if name.contains("(code breaker)") || name.contains("(codebreaker)") {
        Some("codebreaker")
    } else {
        None
    }
}

pub fn region_hint(filename: &str) -> Option<String> {
    for segment in filename.split('(').skip(1) {
        let Some((token, _)) = segment.split_once(')') else {
            continue;
        };
        let token = token.trim();
        let first = token.split(',').next().unwrap_or("").trim().to_ascii_lowercase();
        if [
            "usa", "japan", "europe", "world", "asia", "korea",
            "china", "france", "germany", "spain", "italy", "brazil",
            "australia",
        ].contains(&first.as_str()) {
            return Some(token.to_owned());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_compound_codes_and_sparse_ordinals() {
        let p = parse_cht(
            "cheats = 2\r\ncheat5_desc = \"Jump = 3\"\r\ncheat5_code = \"ABCD-EFGH+1234-5678\"\r\ncheat5_enable = true\r\ncheat8_code = \"0007FA04\"\r\n",
        );
        assert_eq!(p.declared_count, Some(2));
        assert!(p.warnings.is_empty());
        assert_eq!(p.codes[0].ordinal, 5);
        assert_eq!(p.codes[0].description.as_deref(), Some("Jump = 3"));
        assert_eq!(p.codes[0].code.as_deref(), Some("ABCD-EFGH+1234-5678"));
        assert!(p.codes[0].source_enabled);
        assert_eq!(p.codes[0].verification, "unverified");
    }

    #[test]
    fn mismatch_is_warning_not_destructive_edit() {
        let p = parse_cht("cheats = 2\ncheat0_desc = \"Missing\"\ncheat1_code = \"ABCD\"\n");
        assert_eq!(p.codes.len(), 2);
        assert!(p.warnings.is_empty());
        assert!(p.codes[0].code.is_none());
        assert_eq!(p.codes[0].role, "section-heading");
    }

    #[test]
    fn real_entry_count_mismatch_is_reported() {
        let parsed = parse_cht("cheats = 4\ncheat0_desc = \"A group\"\ncheat1_code = \"ABCD\"\n");
        assert_eq!(parsed.warnings.len(), 1);
    }

    #[test]
    fn filename_identity_is_explicitly_advisory() {
        let raw = "Donkey Kong Country (USA) (Game Genie)";
        assert_eq!(title_hint(raw), "Donkey Kong Country");
        assert_eq!(candidate_game_key(&title_hint(raw)), "donkey-kong-country");
        assert_eq!(format_hint(raw), Some("game-genie"));
        assert_eq!(region_hint(raw).as_deref(), Some("USA"));
        assert_eq!(title_hint("Chrono Trigger (Rumbles)"), "Chrono Trigger");
        assert_eq!(title_hint("Something (Special Edition)"), "Something (Special Edition)");
    }
}
