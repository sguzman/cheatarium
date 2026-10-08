# Cartridge and ROM identity evidence

Cheatarium distinguishes **game title suggestions**, **ROM release hash evidence**, and **verified cheat compatibility**. They are separate layers. A source filename is insufficient for the latter two.

## Shipping today

Cheatarium ships `identities/v1/snes.json`, copied into `generated/v1/identities/snes.json` by the index builder and checked by the distribution manifest. It currently contains **zero real release records**. This is intentional: no No-Intro or other third-party ROM hashes have yet been researched, licensed as necessary, attributed, and reviewed. Do not generate 'known' hashes from filename guesses.

The `cheatarium-client::identity` module loads the registry and can calculate SHA-256 over a user-selected local file without altering it. The CLI reports either `no_evidence`, `documented_release_claim`, `conflicting_release_claims`, or `fingerprint_length_conflict`. Every response sets `candidate_only: true` and `cheat_compatibility_verified: false`.

```fish
cargo run --release -p cheatarium-client --bin cheatarium-fingerprint -- --platform snes --file /path/to/your-own-dump.sfc
cargo run --release -p cheatarium-client --bin cheatarium-fingerprint -- --platform snes --sha256 ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
```

The second command uses the SHA-256 of the ASCII text `abc` solely as a test vector, **not** as a Nintendo ROM identity. With the current empty registry both commands produce `no_evidence` unless users supply a file whose hash appears in a future evidence record.

## Strict hash semantics

The hash scope is always `sha256-entire-file-unaltered`. **Do not automatically strip a 512-byte copier header**, trim padding, reverse bytes, strip ROM metadata, or infer headers from file extensions. Such transforms could be supported later as explicitly named and audited normalization algorithms, but a normalized hash must never silently replace a whole-file hash.

Fingerprinting a private file is local-only. No ROM, firmware, or user-provided contents are checked into Cheatarium or uploaded. The fingerprint is a cryptographic digest of the exact bytes, not a copyright clearance or a successful emulation test.

## Registry contract

Each record requires a lowercase 64-character SHA-256 digest, nonzero byte length, game ID, game title, edition ID, and a nonempty list of provenance evidence. Optional region and revision fields may be omitted using JSON `null`.

Each evidence citation requires a HTTPS source URL, a reproducible reference locator (such as table/entry ID), a source revision, and review state `candidate` or `reviewed`. The review state refers to **review of the hash attribution**, not testing a cheat. Different release claims for the same hash are reported as conflicts, never silently selected.

Sample shape (a **non-ROM fixture**, not data to commit into the registry):

```json
{
  "sha256": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
  "byte_length": 3,
  "game_id": "fixture-only",
  "title": "Fixture, not a real ROM",
  "edition_id": "test-vector",
  "region": null,
  "revision": null,
  "evidence": [{
    "url": "https://example.invalid/test-vector",
    "reference": "ASCII abc",
    "source_revision": "test",
    "review_state": "candidate"
  }]
}
```

## Future proof required for compatibility

Even if a user's exact ROM hash matches independently reviewed identity evidence, Cheatarium must separately establish that a particular code has been tested on that same ROM and device/core setting. Until then: show candidate names and original sources, but **never automatically enable cheats**.

To publish genuine hashes, first record provenance and redistribution constraints of the hash collection. Each line needs an auditable source; no scraping copyrighted ROM images or fabricated No-Intro matches. Only the independent emulator consuming Cheatarium can apply a cheat, and it must opt in.
