# Reviewed gameplay-effect evidence

Cheatarium separates **original source descriptions**, **lexical text hints**, **external reports**, **observed behavior in a specific test setup**, and **failed reproduction attempts**. These are not interchangeable.

The editable registry is `reviews/v1/claims.json`. Only independently evidenced claims may enter it. The read-only, checksummed export is `generated/v1/reviews.json`. It now contains **five externally reported claims and zero tested observations**. Collecting or cross-referencing a cheat does not imply it has been tested.

## Assessments

| State | Meaning | Evidence |
| --- | --- | --- |
| `reported` | Attribution-backed, externally reported effect | Original source record and ordinal, review author/date and rationale, HTTPS evidence with revision and entry locator |
| `observed` | Reported gameplay behavior actually observed during a specific test | All source and review references plus exact ROM SHA-256, emulator/version, core/device, date, observed outcome |
| `not-reproduced` | Gameplay behavior not reproduced during a particular test | The same exact-ROM test fields with negative outcome |

An observation is bounded by its test setup. It is not proof that a cheat works across all cartridges, regions or emulators. A failed attempt does not universally prove a cheat is broken.

## Required attribution

Each claim has its own stable `id`, `platform`, `effect_category`, `assessment`, `reviewed_by`, `review_date`, and `assessment_note`. It must also link to the original source occurrence by `source_record_id` and `source_ordinal`, and capture its upstream `source_revision` and `source_git_blob_sha`.

The required `evidence` array contains HTTPS `url`, precise `reference` locator, and `source_revision`. Only `observed` or `not-reproduced` assessments can contain a non-null `test_context`. A test context requires `rom_sha256` (SHA-256 over exact original bytes), `emulator`, `emulator_version`, `code_device_or_core`, `test_date`, `outcome` and `observed_behavior`.

Validation rejects fabricated/missing source IDs, changed blobs, section-heading ordinals, contradictory test outcomes, invalid dates, unknown categories, and unsupported or insufficient evidence. The reviews do not overwrite source descriptions or automatically upgrade curated status values.

Keep private ROM data and absolute local paths outside this public repository; use non-sensitive reviewer identifiers and reproducible public references.

## Offline consumers

```fish
cargo run --release -p cheatarium-client --bin cheatarium-query -- reviews --db generated/v1 --platform snes --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- reviews --db generated/v1 --platform snes --category lives --json
```

The Rust client checks the source-record ID, original code ordinal, source revision and blob, the reviewer claims, and the test context before returning a claim. The results include explicit non-activation flags. Consumers must pin the trusted distribution commit.

To validate after building the source bundles, run `python3 tools/validate_effect_reviews.py --write`, followed by `--check`. CI also runs synthetic positive and adversarial tests, then checksums the exported registry.

## First sourced reports

As of October 8, 2026, five `reported` claims link specific original entries to two dated historical GameFAQs documents:

- Three SNES **Donkey Kong Country** codes: `1DCC-CA7A` (almost invincible), `C2C1-4A9C` (infinite lives), and `A081-1273` (Donkey Kong high jump). See [CStassen's DKC FAQ v1.3, March 27, 1995](https://gamefaqs.gamespot.com/snes/588282-donkey-kong-country/faqs/5450), Q-11. That FAQ *reprints historical Lewis Galoob code-update text*; it is not a new empirical test. It explicitly distinguishes **alternate codes for two game versions**, without identifying corresponding ROM SHA-256 fingerprints. "Almost invincible" includes a stuck-state warning. These caveats are preserved in each review.
- Two NES **Donkey Kong** codes: `SXNGOZVG` (infinite lives), and `PENKNPLE` (start with nine lives). See [MaineCane's Donkey Kong guide v0.9, December 30, 2004](https://gamefaqs.gamespot.com/nes/563403-donkey-kong/faqs/34396), Section IV. The latter is **not** an infinite-lives claim.

Both documents independently exist outside the Libretro archive, but the historical cheat lists may share original code authorship. This counts as **external source corroboration of a reported code effect**, not independent demonstration, nor proof of cartridge revision compatibility. No ROMs were accessed, and source material is linked rather than copied into Cheatarium.

### A source encoding hazard to review

The raw Libretro DKC Game Genie file contains `C2C9-4E2C+C2C1-4A9C` as one combined source string, while the historical FAQ calls these **alternative codes for different game versions**, not a simultaneous two-code combination. That difference must be resolved with source-specific semantics before any exporter or emulator decides to interpret or execute the combined string. The raw archive has deliberately not been altered.

## Next step

Collect more independently attributable references, distinguish version alternatives from multi-code combinations, and acquire reproducible exact-ROM test observations. Do not manufacture observations from lexical categories, filenames, decoded addresses, or source listings alone.
