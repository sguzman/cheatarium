//! Source-preserving views of non-Libretro native console cheat formats.
//! Original archived files remain authoritative. No code is interpreted or run.
use crate::{Code, NativeField, ParsedCheats};

fn field(name: &str, value: impl Into<String>) -> NativeField {
    NativeField { name: name.to_owned(), value: value.into() }
}

fn result() -> ParsedCheats {
    ParsedCheats { declared_count: None, codes: Vec::new(), warnings: Vec::new() }
}

fn add(out: &mut ParsedCheats, name: String, payload: Option<String>, metadata: Vec<NativeField>) {
    let role = if payload.as_ref().is_some_and(|s| !s.trim().is_empty()) {
        "code"
    } else { "section-heading" };
    out.codes.push(Code {
        ordinal: out.codes.len(), description: Some(name),
        code: payload.filter(|s| !s.trim().is_empty()), source_enabled: false,
        verification: "unverified", role, native_fields: metadata,
        snes_decode: None, composition: None,
    });
}

pub fn parse_artemis_ncl(text: &str) -> ParsedCheats {
    let mut out = result();
    let mut lines = Vec::<String>::new();
    let mut flush = |lines: &mut Vec<String>, out: &mut ParsedCheats| {
        if lines.is_empty() { return; }
        let name = lines.first().map_or("", String::as_str).trim();
        if name.is_empty() {
            out.warnings.push("NCL block has an empty description".into());
        }
        let parameter = lines.get(1).map_or("", String::as_str).trim().to_owned();
        let author = lines.get(2).map_or("", String::as_str).trim().to_owned();
        let payload = if lines.len() > 3 { Some(lines[3..].join("\n") + "\n") } else { None };
        add(out, if name.is_empty() { "(unnamed NCL entry)".into() } else { name.to_owned() },
            payload, vec![field("original_parameter", parameter), field("original_author", author)]);
        lines.clear();
    };
    for line in text.lines() {
        if line.trim() == "#" { flush(&mut lines, &mut out); }
        else { lines.push(line.to_owned()); }
    }
    flush(&mut lines, &mut out);
    out
}

pub fn parse_goldhen_json(text: &str) -> ParsedCheats {
    let mut out = result();
    match serde_json::from_str::<serde_json::Value>(text) {
        Ok(value) => {
            let credits = value.get("credits").map_or("".into(), |v| v.to_string());
            let process = value.get("process").and_then(|v| v.as_str()).unwrap_or("").to_owned();
            if let Some(mods) = value.get("mods").and_then(|v| v.as_array()) {
                for item in mods {
                    let name = item.get("name").and_then(|v| v.as_str())
                        .unwrap_or("(unnamed native mod)").to_owned();
                    add(&mut out, name, Some(item.to_string()), vec![
                        field("original_process", process.clone()), field("original_credits", credits.clone()),
                        field("native_type", item.get("type").and_then(|v| v.as_str()).unwrap_or("")),
                    ]);
                }
            } else {
                out.warnings.push("GoldHEN JSON has no mods array".into());
                add(&mut out, "(original JSON metadata)".into(), None,
                    vec![field("original_source_text", text)]);
            }
        }
        Err(e) => {
            out.warnings.push(format!("GoldHEN JSON parse failed: {e}"));
            add(&mut out, "(unparsed original JSON)".into(), None,
                vec![field("original_source_text", text)]);
        }
    }
    out
}

pub fn parse_goldhen_shn(text: &str) -> ParsedCheats {
    let mut out = result();
    let mut rest = text;
    while let Some(start) = rest.find("<Cheat ") {
        rest = &rest[start..];
        let Some(end) = rest.find("</Cheat>") else {
            out.warnings.push("Unterminated original SHN Cheat element".into());
            break;
        };
        let close = end + "</Cheat>".len();
        let raw = &rest[..close];
        let opening = raw.split_once('>').map_or("", |pair| pair.0);
        let name = opening.split("Text=\"").nth(1)
            .and_then(|s| s.split_once('"').map(|p| p.0))
            .unwrap_or("(unnamed SHN entry)");
        add(&mut out, name.to_owned(), Some(raw.to_owned()), vec![field("native_format", "GoldHEN SHN XML")]);
        rest = &rest[close..];
    }
    if out.codes.is_empty() {
        out.warnings.push("No parseable SHN Cheat elements; preserved original text".into());
        add(&mut out, "(original SHN source)".into(), None,
            vec![field("original_source_text", text)]);
    }
    out
}

pub fn parse_goldhen_mc4(text: &str) -> ParsedCheats {
    let mut out = result();
    add(&mut out, "(opaque original MC4 source)".into(), None,
        vec![field("encoded_original_source", text)]);
    out.warnings.push("Original MC4 is opaque: not decoded or executable".into());
    out
}

pub fn parse_gecko_ini(text: &str) -> ParsedCheats {
    let mut out = result();
    let mut mode = String::new();
    let mut title: Option<String> = None;
    let mut body = String::new();
    fn flush(out: &mut ParsedCheats, title: &mut Option<String>, body: &mut String, mode: &str) {
        if let Some(name) = title.take() {
            let payload = std::mem::take(body);
            add(out, name, if payload.trim().is_empty() { None } else { Some(payload) },
                vec![field("original_ini_section", mode)]);
        }
    }
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            flush(&mut out, &mut title, &mut body, &mode);
            mode = trimmed.trim_matches(&['[', ']'][..]).to_owned();
        } else if let Some(name) = trimmed.strip_prefix('$') {
            flush(&mut out, &mut title, &mut body, &mode);
            title = Some(name.to_owned());
        } else if title.is_some() { body.push_str(line); }
    }
    flush(&mut out, &mut title, &mut body, &mode);
    if out.codes.is_empty() { out.warnings.push("No original Dolphin dollar-prefixed entries".into()); }
    out
}

pub fn parse_gecko_markdown(text: &str) -> ParsedCheats {
    let mut out = result();
    let mut title = "(original Gecko code document)".to_owned();
    let mut revision = String::new();
    let mut code = String::new();
    let mut fenced = false;
    const FENCE: &str = "\x60\x60\x60";
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("## ") { title = heading.to_owned(); }
        if let Some(summary) = trimmed.strip_prefix("<summary>")
            .and_then(|s| s.strip_suffix("</summary>")) { revision = summary.to_owned(); }
        if trimmed.starts_with(FENCE) {
            if fenced {
                add(&mut out, title.clone(), Some(std::mem::take(&mut code)),
                    vec![field("original_revision_heading", revision.clone())]);
            }
            fenced = !fenced;
        } else if fenced {
            code.push_str(line);
        }
    }
    if fenced { out.warnings.push("Unterminated source Markdown code fence".into()); }
    if out.codes.is_empty() {
        out.warnings.push("No fenced Gecko code in original Markdown".into());
        add(&mut out, title, None, vec![field("original_source_markdown", text)]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ncl_keeps_author_and_native_payload_without_merging_sections() {
        let p = parse_artemis_ncl("Infinite Life\n0\nDANNY G\n0 001FCC10 60000000\n#\nMax Money\n0\nMedo\n0 30CF7180 3B9AC9FF\n#");
        assert_eq!(p.codes.len(), 2);
        assert_eq!(p.codes[0].native_fields[1].value, "DANNY G");
        assert_eq!(p.codes[0].code.as_deref(), Some("0 001FCC10 60000000\n"));
        assert_eq!(p.codes[1].ordinal, 1);
        assert_eq!(p.codes[1].verification, "unverified");
    }

    #[test]
    fn goldhen_json_preserves_author_and_native_memory_payload() {
        let json = r#"{"credits":["Talixme"],"mods":[{"name":"Godmode","type":"checkbox","memory":[{"offset":"668DA3","on":"90909090"}]}]}"#;
        let p = parse_goldhen_json(json);
        assert_eq!(p.codes.len(), 1);
        assert!(p.codes[0].code.as_ref().unwrap().contains("668DA3"));
        assert!(p.codes[0].native_fields[1].value.contains("Talixme"));
        assert!(!p.codes[0].source_enabled);
    }

    #[test]
    fn mc4_is_not_fabricated_as_a_decoded_cheat() {
        let p = parse_goldhen_mc4("opaque");
        assert_eq!(p.codes[0].role, "section-heading");
        assert!(p.codes[0].code.is_none());
        assert_eq!(p.codes[0].native_fields[0].value, "opaque");
    }

    #[test]
    fn shn_xml_entries_are_kept_separate() {
        let p = parse_goldhen_shn("<Cheat Text=\"A\"><Cheatline /></Cheat><Cheat Text=\"B\"><Cheatline /></Cheat>");
        assert_eq!(p.codes.len(), 2);
        assert_eq!(p.codes[1].description.as_deref(), Some("B"));
    }

    #[test]
    fn gecko_ini_keeps_device_and_original_lines() {
        let p = parse_gecko_ini("[ActionReplay]\n$Lives\n04001234 00000063\n[Gecko]\n$Coins\n04000000 000000FF\n");
        assert_eq!(p.codes.len(), 2);
        assert_eq!(p.codes[0].native_fields[0].value, "ActionReplay");
        assert_eq!(p.codes[1].native_fields[0].value, "Gecko");
    }

    #[test]
    fn gecko_markdown_keeps_distinct_revision_blocks() {
        let md = "## Save Anytime\n<summary>USA</summary>\n\x60\x60\x60hex\n04000000 00000001\n\x60\x60\x60\n<summary>PAL</summary>\n\x60\x60\x60hex\n04000000 00000002\n\x60\x60\x60\n";
        let p = parse_gecko_markdown(md);
        assert_eq!(p.codes.len(), 2);
        assert_eq!(p.codes[0].native_fields[0].value, "USA");
        assert_eq!(p.codes[1].native_fields[0].value, "PAL");
        assert_ne!(p.codes[0].code, p.codes[1].code);
    }
}
