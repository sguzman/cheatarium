#!/usr/bin/env python3
"""Read-only index of pinned public-domain retro homebrew built-in secrets.

The original JavaScript is never evaluated. Button sequences are source
claims, not tested gameplay or memory-write codes.
"""
import argparse
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = "retro-homebrew-secrets"
OUTPUT = ROOT / "generated/v1/secrets/homebrew.json"
FIELDS = ("gameTitle", "developer", "consoleTags", "cheatTypeTags", "cheatEffect",
          "inputSequence", "whenToInput", "activationIndicator", "dateAdded")
PLATFORMS = {
    "Game Boy": "gb", "Game Boy Color": "gbc",
    "Game Boy Advance": "gba", "GameGear": "game-gear",
    "MegaDrive/Genesis": "genesis", "NES": "nes",
    "SMS": "master-system", "SNES": "snes",
}


def blob_sha(raw):
    return hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()


def parse_original(text):
    m = re.fullmatch(r"\s*const\s+jsonCheatEntries\s*=\s*(\[.*\])\s*;\s*", text, re.S)
    if not m:
        raise ValueError("Unexpected source wrapper; never evaluate JavaScript")
    body = m.group(1)
    out = []
    quoted = escape = False
    for i, ch in enumerate(body):
        if quoted:
            out.append(ch)
            if escape:
                escape = False
            elif ch == "\\":
                escape = True
            elif ch == '"':
                quoted = False
        elif ch == '"':
            quoted = True
            out.append(ch)
        elif ch == "," and next((c for c in body[i + 1:] if not c.isspace()), "") in ("}", "]"):
            continue
        else:
            out.append(ch)
    if quoted:
        raise ValueError("Unclosed source string")
    data = json.loads("".join(out))
    if not isinstance(data, list):
        raise ValueError("Source must contain an array")
    return data


def build(root=ROOT):
    manifest = json.loads((root / "sources/retro-homebrew-secrets.json").read_text("utf-8"))
    if (manifest.get("id") != SOURCE or manifest.get("schema_version") != 1
            or manifest.get("license") != "Unlicense / public domain data"
            or manifest.get("entry_type") != "built-in-secret-not-memory-cheat"
            or manifest.get("source_entry_count") != 19):
        raise ValueError("Wrong source, rights, or pinned source count")
    originals = {}
    for record in manifest["files"]:
        name = record["upstream_path"]
        if name not in ("README.md", "LICENSE", "data/cheat_entries.js") or name in originals:
            raise ValueError("Unexpected original file")
        if record["archive_path"] != "archive/" + SOURCE + "/" + name:
            raise ValueError("Original path changed")
        raw = (root / record["archive_path"]).read_bytes()
        if blob_sha(raw) != record["git_blob_sha"]:
            raise ValueError("Original Git blob mismatch")
        originals[name] = raw.decode("utf-8")
    if set(originals) != {"README.md", "LICENSE", "data/cheat_entries.js"}:
        raise ValueError("Original README, data or license missing")
    if ("All data contained and submitted to this repo is Public Domain."
            not in originals["README.md"]
            or "free and unencumbered software released into the public domain"
            not in originals["LICENSE"]):
        raise ValueError("Public-domain notices changed")
    raw_records = parse_original(originals["data/cheat_entries.js"])
    if len(raw_records) != manifest["source_entry_count"]:
        raise ValueError("Original entry count changed")
    known = {p["id"] for p in json.loads((root / "platforms/platforms.json").read_text("utf-8"))["platforms"]}
    records = []
    for i, raw in enumerate(raw_records):
        if not isinstance(raw, dict) or set(raw) != set(FIELDS) or any(not isinstance(raw[k], str) for k in FIELDS):
            raise ValueError("Unexpected original entry format")
        if not all(raw[k].strip() for k in ("gameTitle", "developer", "consoleTags",
                                            "cheatEffect", "inputSequence", "whenToInput", "dateAdded")):
            raise ValueError("Incomplete native entry")
        labels = [p.strip() for p in raw["consoleTags"].split(",")]
        if not labels or len(set(labels)) != len(labels) or any(p not in PLATFORMS or PLATFORMS[p] not in known for p in labels):
            raise ValueError("New source platform requires review")
        if not re.fullmatch(r"\d{4}-\d{2}-\d{2}", raw["dateAdded"]):
            raise ValueError("Unexpected date format")
        records.append({
            "source_ordinal": i, "game_title_source_claim": raw["gameTitle"],
            "developer_source_claim": raw["developer"],
            "original_platform_labels": labels, "platforms": [PLATFORMS[p] for p in labels],
            "original_cheat_type_tags": raw["cheatTypeTags"],
            "effect_source_claim": raw["cheatEffect"],
            "input_sequence_source_claim": raw["inputSequence"],
            "when_to_input_source_claim": raw["whenToInput"],
            "activation_indicator_source_claim": raw["activationIndicator"],
            "source_date_added": raw["dateAdded"],
            "destructive_source_signal": "Clear Save" in raw["cheatTypeTags"],
            "effect_verified": False, "execution_supported": False,
        })
    return {
        "schema_version": 1, "format": "cheatarium-homebrew-built-in-secrets-v1",
        "source_id": SOURCE, "source_repository": manifest["repository"],
        "source_commit": manifest["snapshot_commit"],
        "source_git_blob_sha": next(x["git_blob_sha"] for x in manifest["files"] if x["upstream_path"] == "data/cheat_entries.js"),
        "license": manifest["license"], "metadata_or_code_execution": False,
        "built_in_input_sequences_not_memory_writes": True,
        "game_identity_or_effect_verified": False,
        "source_entries": len(records),
        "distinct_source_game_title_labels": len({r["game_title_source_claim"] for r in records}),
        "records": records,
    }


def search(report, text, platform=None):
    text = text.casefold()
    return [r for r in report["records"] if
            (platform is None or platform in r["platforms"]) and
            any(text in r[k].casefold() for k in (
                "game_title_source_claim", "developer_source_claim",
                "effect_source_claim", "input_sequence_source_claim"))]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    modes = p.add_mutually_exclusive_group(required=True)
    modes.add_argument("--write", action="store_true")
    modes.add_argument("--check", action="store_true")
    modes.add_argument("--search")
    p.add_argument("--platform")
    args = p.parse_args()
    report = build()
    if args.search is not None:
        rows = search(report, args.search, args.platform)
        print(json.dumps({"no_auto_activation": True, "gameplay_verified": False,
                          "total": len(rows), "records": rows}, ensure_ascii=False))
        return
    rendered = json.dumps(report, indent=2, ensure_ascii=False) + "\n"
    if args.write:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(rendered, "utf-8")
    elif not OUTPUT.exists() or OUTPUT.read_text("utf-8") != rendered:
        raise SystemExit("Stale or modified homebrew built-in secrets index")
    print("OK: public-domain built-in secrets", report["source_entries"])


if __name__ == "__main__":
    main()
