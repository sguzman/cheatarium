# Gameplay-effect text signals

Cheatarium can filter source cheat descriptions into broad gameplay topics using **explicit, reproducible lexical phrases**. This is not gameplay classification backed by tests. The aim is to make very large original code dumps discoverable while retaining complete source evidence.

## Versioned taxonomy

The source of truth is [`taxonomy/effects-v1.json`](../taxonomy/effects-v1.json), which currently defines 11 topics: lives, health, invulnerability, time, ammunition, currency, score, energy-magic, unlocking, items-equipment and movement. Each topic lists a small set of English text phrases. Both taxonomy and deterministic compressed per-console result indexes are included in the published SHA-256 distribution.

The matching algorithm splits description text into ASCII alphanumeric words, lowercases them and recognizes only a contiguous *whole-token* phrase. `INFINITE  LIVES!` matches `infinite lives`; `notinfinite lives` does not. The first matching phrase for a category is recorded; the same description can match more than one category.

Only original descriptions of `code` and `memory-entry` records are considered. Filenames, guessed title identities, SNES syntax decodes, cheat binary values and section headings cannot trigger a category tag. Every match retains `source_record_id`, `ordinal`, `candidate_game_key` (advisory only) and `matched_phrase`. Resolve the source record in `generated/v1/<platform>.json.gz` for the exact original description, device code, filename and attribution.

## Offline queries

```fish
cargo run --release -p cheatarium-client --bin cheatarium-query -- tags --db generated/v1 --platform snes --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- tags --db generated/v1 --platform snes --category lives --limit 10 --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- tags --db generated/v1 --platform snes --category health --game-key super-mario-world --json
```

Without a category the CLI returns counts, not hundreds of thousands of entries. The category form returns a bounded sample, total matched occurrence count, source links and explicit `source_text_only: true`, `verified_effect: false`, `verified_game_identity: false`, and `cheats_activated: false`.

## Interpretation limits

This is an **English-only language filter**, not an ontological conclusion. A description like `does not give infinite lives` still contains the phrase `infinite lives`; source authors may write contradictory, sarcastic, misleading or erroneous descriptions. `energy` may mean different mechanics in different games, and a cheat might require precise ROM revisions. A tag means *this original description contains this phrase*, nothing more.

Cheatarium's full code archive, original code ordinal, device formatting and provenance are untouched. The index is reproducible from those descriptions, and `tools/build_distribution.py` independently recomputes every emitted tag and phrase. Semantic interpretations or code equivalence require a separately reviewed evidence layer.
