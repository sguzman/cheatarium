//! Loss-tolerant indexing of original Libretro .cht records.
//!
//! This code does NOT decode a Game Genie/Action Replay code into executable
//! emulator memory writes. Exact original bytes remain in archive/.
//! Filename-based game associations are suggestions, never ROM verification.
pub mod effect_signals;
use cheatarium_codecs::SnesDecoded;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Default)]
struct PartialCheat {
    description: Option<String>,
    code: Option<String>,
    source_enabled: bool,
    native_fields: Vec<NativeField>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NativeField {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Code {
    pub ordinal: usize,
    pub description: Option<String>,
    pub code: Option<String>,
    pub source_enabled: bool,
    pub verification: &'static str,
    pub role: &'static str,
    pub native_fields: Vec<NativeField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snes_decode: Option<SnesDecoded>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composition: Option<SourceComposition>,
}

/// Literal '+' in a source file does not establish simultaneous execution.
/// An evidenced version-alternative partition is not a mapping to a known ROM.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceComposition {
    pub relation: String,
    pub alternatives: Vec<Vec<String>>,
    pub evidence: Vec<CompositionEvidence>,
    pub rom_match_verified: bool,
    pub simultaneous_execution_confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompositionEvidence {
    pub url: String,
    pub reference: String,
    pub source_revision: String,
}

impl SourceComposition {
    #[must_use]
    pub fn unresolved() -> Self {
        Self {
            relation: "unresolved".to_owned(),
            alternatives: Vec::new(),
            evidence: Vec::new(),
            rom_match_verified: false,
            simultaneous_execution_confirmed: false,
        }
    }
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

/// Preserve original bracketed Gateshark/Atmosphère cheat sections as
/// indivisible, unverified multiline source-code occurrences.
///
/// This is a grouping parser, not a code interpreter. It does not infer
/// executable writes, disabled/enabled state, game names, or ROM compatibility.
/// Complete original bytes remain in the separately archived source file.
pub fn parse_native_sections(text: &str) -> ParsedCheats {
    fn append_section(codes: &mut Vec<Code>, description: Option<String>, body: String) {
        let is_code = !body.trim().is_empty() && description.is_some();
        let native_fields = if description.is_none() && !body.trim().is_empty() {
            vec![NativeField {
                name: "unattributed_original_text".to_owned(),
                value: body,
            }]
        } else {
            Vec::new()
        };
        let code = if is_code { Some(body) } else { None };
        if description.is_some() || !native_fields.is_empty() {
            codes.push(Code {
                ordinal: codes.len(),
                description: description.or_else(|| Some("(unsectioned original source text)".into())),
                code,
                source_enabled: false,
                verification: "unverified",
                role: if is_code { "code" } else { "section-heading" },
                native_fields,
                snes_decode: None,
                composition: None,
            });
        }
    }

    let mut codes = Vec::new();
    let mut title: Option<String> = None;
    let mut body = String::new();
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        let heading = trimmed
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .filter(|s| !s.is_empty() && !s.contains(['\r', '\n']));
        if let Some(name) = heading {
            append_section(&mut codes, title.take(), std::mem::take(&mut body));
            title = Some(name.to_owned());
        } else {
            body.push_str(line);
        }
    }
    append_section(&mut codes, title, body);
    ParsedCheats {
        declared_count: None,
        codes,
        warnings: Vec::new(),
    }
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
            _ => entry.native_fields.push(NativeField {
                name: field.to_owned(),
                value,
            }),
        }
    }

    let codes: Vec<_> = cheats
        .into_iter()
        .map(|(ordinal, part)| {
            let encoded = part.code.as_deref().is_some_and(|s| !s.trim().is_empty());
            let has_address = part
                .native_fields
                .iter()
                .any(|f| f.name == "address" && !f.value.is_empty());
            let has_value = part
                .native_fields
                .iter()
                .any(|f| f.name == "value" && !f.value.is_empty());
            let role = if encoded {
                "code"
            } else if has_address && has_value {
                "memory-entry"
            } else {
                "section-heading"
            };
            Code {
                ordinal,
                description: part.description,
                role,
                code: part.code.filter(|s| !s.trim().is_empty()),
                source_enabled: part.source_enabled,
                verification: "unverified",
                native_fields: part.native_fields,
                snes_decode: None,
                composition: None,
            }
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
    ParsedCheats {
        declared_count,
        codes,
        warnings,
    }
}

/// Parenthetical suffixes stripped here are only common *filename metadata*.
/// The full source filename and raw upstream path are always exported too.
fn is_metadata_suffix(inner: &str) -> bool {
    let lower = inner.trim().to_ascii_lowercase();
    const EXACT: &[&str] = &[
        "usa",
        "japan",
        "europe",
        "world",
        "asia",
        "korea",
        "china",
        "france",
        "germany",
        "spain",
        "italy",
        "brazil",
        "australia",
        "u",
        "e",
        "j",
        "game genie",
        "action replay",
        "pro action replay",
        "gameshark",
        "game shark",
        "code breaker",
        "codebreaker",
        "rumbles",
        "diff",
        "unl",
        "beta",
        "proto",
        "prototype",
        "virtual console",
        "sgb enhanced",
        "gbc",
        "gba",
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
        let first = token
            .split(',')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if [
            "usa",
            "japan",
            "europe",
            "world",
            "asia",
            "korea",
            "china",
            "france",
            "germany",
            "spain",
            "italy",
            "brazil",
            "australia",
        ]
        .contains(&first.as_str())
        {
            return Some(token.to_owned());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_section_parser_preserves_multiline_codes_and_original_ordinals() {
        let text = "[Max Health]\r\nDD000000 00000280\r\nD3000000 144276F4\r\n\r\n"
            .to_owned()
            + "[Infinite Coins]\n11160000 5C3BE7DC 00000000\n"
            + "[Heading only]\n";
        let parsed = parse_native_sections(&text);
        assert_eq!(parsed.codes.len(), 3);
        assert_eq!(parsed.codes[0].ordinal, 0);
        assert_eq!(parsed.codes[0].description.as_deref(), Some("Max Health"));
        assert_eq!(
            parsed.codes[0].code.as_deref(),
            Some("DD000000 00000280\r\nD3000000 144276F4\r\n\r\n")
        );
        assert!(!parsed.codes[0].source_enabled);
        assert_eq!(parsed.codes[0].role, "code");
        assert_eq!(parsed.codes[0].verification, "unverified");
        assert_eq!(parsed.codes[1].code.as_deref(), Some("11160000 5C3BE7DC 00000000\n"));
        assert_eq!(parsed.codes[2].role, "section-heading");
        assert!(parsed.codes[2].code.is_none());
    }

    #[test]
    fn native_section_parser_keeps_unlabeled_source_text_without_fake_code() {
        let parsed = parse_native_sections("// original source comment\r\n[Cheat]\n01000000 ABCD\n");
        assert_eq!(parsed.codes.len(), 2);
        assert_eq!(parsed.codes[0].role, "section-heading");
        assert!(parsed.codes[0].code.is_none());
        assert_eq!(parsed.codes[0].native_fields[0].value, "// original source comment\r\n");
        assert_eq!(parsed.codes[1].ordinal, 1);
        assert_eq!(parsed.codes[1].description.as_deref(), Some("Cheat"));
        assert_eq!(parsed.codes[1].code.as_deref(), Some("01000000 ABCD\n"));
        assert!(parse_native_sections("").codes.is_empty());
    }

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
    fn preserves_native_memory_entries_without_fake_code_strings() {
        let parsed = parse_cht("cheats = \"1\"\ncheat0_desc = \"Infinite Lives\"\ncheat0_address = \"38\"\ncheat0_value = \"8\"\ncheat0_cheat_type = \"1\"\n");
        assert!(parsed.warnings.is_empty());
        assert_eq!(parsed.codes[0].role, "memory-entry");
        assert!(parsed.codes[0].code.is_none());
        assert_eq!(parsed.codes[0].native_fields.len(), 3);
        assert_eq!(parsed.codes[0].native_fields[0].name, "address");
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
        assert_eq!(
            title_hint("Something (Special Edition)"),
            "Something (Special Edition)"
        );
    }
}
