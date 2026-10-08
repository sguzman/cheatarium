# Reviewed gameplay-effect evidence

Cheatarium separates **original source descriptions**, **lexical text hints**, **external reports**, **observed behavior in a specific test setup**, and **failed reproduction attempts**. These are not interchangeable.

The editable registry is `reviews/v1/claims.json`. Only independently evidenced claims may enter it. The read-only, checksummed export is `generated/v1/reviews.json`. It initially contains **zero claims**: collecting a cheat never implies it has been tested.

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

## Next step

Populate this register through real independent research or controlled tests. Do not manufacture observations from a lexical category, filename, raw code string, or decoded address.
