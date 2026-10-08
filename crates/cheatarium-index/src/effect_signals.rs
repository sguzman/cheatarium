//! Strict lexical cues in original cheat descriptions. Not inferred effects.
//! English-only, ASCII word-boundary phrase matching; never tests cheat codes.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EffectTaxonomy {
    pub schema_version: u32,
    pub id: String,
    pub language: String,
    pub method: String,
    pub interpretation: String,
    pub categories: Vec<EffectCategory>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EffectCategory {
    pub id: String,
    pub label: String,
    pub phrases: Vec<String>,
}

fn ascii_words(text: &str) -> String {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

impl EffectTaxonomy {
    /// Reject changed matching semantics and ambiguous duplicated phrases.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != 1
            || self.id != "cheatarium-effect-signals-en-v1"
            || self.language != "en"
            || self.method != "ascii-token-phrase"
            || self.categories.is_empty()
        {
            return Err("Unsupported lexical effect taxonomy");
        }
        let mut ids = BTreeSet::new();
        for category in &self.categories {
            if category.id.is_empty()
                || !category.id.bytes().all(|b| b.is_ascii_lowercase() || b == b'-')
                || category.label.trim().is_empty()
                || !ids.insert(category.id.as_str())
                || category.phrases.is_empty()
            {
                return Err("Malformed effect category");
            }
            let mut phrases = BTreeSet::new();
            for phrase in &category.phrases {
                if phrase != &ascii_words(phrase) || phrase.is_empty()
                    || !phrases.insert(phrase.as_str())
                {
                    return Err("Malformed or repeated effect signal phrase");
                }
            }
        }
        Ok(())
    }
}

/// A category can have several matching phrases. Only the earliest declared
/// matching phrase is retained, as reproducible *literal language evidence*.
/// No signal is ever applied to an unlabelled code without a description.
#[must_use]
pub fn classify<'a>(
    description: &str,
    taxonomy: &'a EffectTaxonomy,
) -> Vec<(&'a str, &'a str)> {
    let words = ascii_words(description);
    if words.is_empty() {
        return Vec::new();
    }
    let haystack = format!(" {words} ");
    taxonomy.categories.iter()
        .filter_map(|category| {
            category.phrases.iter()
                .find(|phrase| haystack.contains(&format!(" {phrase} ")))
                .map(|phrase| (category.id.as_str(), phrase.as_str()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> EffectTaxonomy {
        serde_json::from_str(include_str!("../../../taxonomy/effects-v1.json")).unwrap()
    }
    #[test]
    fn taxonomy_is_valid_and_stable() {
        fixture().validate().unwrap();
    }
    #[test]
    fn strict_word_boundaries_and_case_insensitive() {
        let t = fixture();
        assert_eq!(classify("INFINITE   LIVES!", &t), vec![("lives", "infinite lives")]);
        assert!(classify("notinfinite lives", &t).is_empty());
        assert!(classify("some invincibilityish text", &t).is_empty());
        assert!(classify("", &t).is_empty());
    }
    #[test]
    fn evidence_is_literal_not_a_functional_claim() {
        let t = fixture();
        let matches = classify("No Damage + Infinite HP", &t);
        assert!(matches.contains(&("invulnerability", "no damage")));
        assert!(matches.contains(&("health", "infinite hp")));
        // A negated statement still contains the exact phrase, and must be
        // presented as a textual cue rather than proven cheat behavior.
        assert!(classify("Does not give infinite lives", &t)
            .contains(&("lives", "infinite lives")));
    }
    #[test]
    fn repeated_category_phrases_are_rejected() {
        let mut t = fixture();
        t.categories[0].phrases.push("infinite lives".to_string());
        assert!(t.validate().is_err());
        t.categories[0].phrases.pop();
        t.categories[0].phrases.push("Infinite Lives".to_string());
        assert!(t.validate().is_err());
    }
}
