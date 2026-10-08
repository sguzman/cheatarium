#!/usr/bin/env python3
"""Validate and publish separately evidenced cheat-effect reviews.

An archived description, a matching phrase, or a decoded address never
constitutes verified gameplay. Only source-bound, documented evidence belongs
in this additive review layer. No ROMs or emulator execution involved.
"""
import argparse
from datetime import date
import gzip
import json
from pathlib import Path
import re
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[1]
SOURCE = Path("reviews/v1/claims.json")
EXPORT = Path("generated/v1/reviews.json")
HEX_SHA = re.compile(r"[0-9a-f]{64}\Z")
ID = re.compile(r"[a-z0-9]+(?:-[a-z0-9]+)*\Z")
ALLOWED = {"reported", "observed", "not-reproduced"}

def require(ok, message):
    if not ok:
        raise ValueError(message)

def valid_date(value):
    try:
        return isinstance(value, str) and date.fromisoformat(value).isoformat() == value
    except ValueError:
        return False

def https_url(value):
    if not isinstance(value, str) or not value.startswith("https://"):
        return False
    url = urlsplit(value)
    return bool(url.hostname) and not url.username and not url.password

def nonblank(value):
    return isinstance(value, str) and bool(value.strip())

def original_source(root, platform, record_id, caches):
    if platform not in caches:
        path = root / "generated/v1" / f"{platform}.json.gz"
        with gzip.open(path, "rt", encoding="utf-8") as f:
            bundle = json.load(f)
        require(bundle.get("platform") == platform and bundle.get("schema_version") == 1,
                f"Invalid source bundle for {platform}")
        records = bundle.get("records", [])
        caches[platform] = {rec["id"]: rec for rec in records}
        require(len(caches[platform]) == len(records), f"Duplicate source records in {platform}")
    record = caches[platform].get(record_id)
    require(record is not None, f"No indexed original source record for {platform} / {record_id}")
    return record

def validate(root: Path):
    review_data = json.loads((root / SOURCE).read_text(encoding="utf-8"))
    require(review_data.get("schema_version") == 1
            and review_data.get("format") == "cheatarium-effect-reviews-v1"
            and isinstance(review_data.get("claims"), list),
            "Unsupported effect-review registry")
    catalog = json.loads((root / "generated/v1/catalog.json").read_text(encoding="utf-8"))
    platforms = {item["platform"] for item in catalog["bundles"]}
    taxonomy = json.loads((root / "taxonomy/effects-v1.json").read_text(encoding="utf-8"))
    categories = {item["id"] for item in taxonomy["categories"]}
    seen_ids = set()
    caches = {}
    for claim in review_data["claims"]:
        require(isinstance(claim, dict), "Non-object effect claim")
        cid = claim.get("id")
        require(isinstance(cid, str) and ID.fullmatch(cid) and cid not in seen_ids,
                "Empty, malformed or duplicate effect-review ID")
        seen_ids.add(cid)
        require(claim.get("platform") in platforms, f"{cid}: unknown platform")
        require(claim.get("effect_category") in categories, f"{cid}: unknown category")
        require(claim.get("assessment") in ALLOWED, f"{cid}: unknown assessment")
        require(nonblank(claim.get("reviewed_by")) and valid_date(claim.get("review_date"))
                and nonblank(claim.get("assessment_note")),
                f"{cid}: no reviewer, review date or assessment rationale")
        record_id = claim.get("source_record_id")
        ordinal = claim.get("source_ordinal")
        require(nonblank(record_id) and type(ordinal) is int and ordinal >= 0,
                f"{cid}: invalid source reference")
        source = original_source(root, claim["platform"], record_id, caches)
        prov = source["provenance"]
        require(claim.get("source_revision") == prov["revision"]
                and claim.get("source_git_blob_sha") == prov["git_blob_sha"],
                f"{cid}: upstream source revision/blob mismatch")
        codes = [c for c in source["codes"] if c["ordinal"] == ordinal]
        require(len(codes) == 1 and codes[0]["role"] in ("code", "memory-entry"),
                f"{cid}: original code entry missing or non-code heading")
        references = claim.get("evidence")
        require(isinstance(references, list) and len(references) > 0,
                f"{cid}: evidence required")
        for e in references:
            require(isinstance(e, dict) and https_url(e.get("url"))
                    and nonblank(e.get("reference"))
                    and nonblank(e.get("source_revision")),
                    f"{cid}: invalid attribution / evidence locator")
        ctx = claim.get("test_context")
        if claim["assessment"] == "reported":
            require(ctx is None, f"{cid}: reported claims cannot imply completed testing")
        else:
            require(isinstance(ctx, dict), f"{cid}: observation requires a test context")
            require(isinstance(ctx.get("rom_sha256"), str)
                    and HEX_SHA.fullmatch(ctx["rom_sha256"])
                    and nonblank(ctx.get("emulator"))
                    and nonblank(ctx.get("emulator_version"))
                    and nonblank(ctx.get("code_device_or_core"))
                    and valid_date(ctx.get("test_date"))
                    and nonblank(ctx.get("observed_behavior")),
                    f"{cid}: incomplete exact-ROM observation environment")
            require(ctx.get("outcome") ==
                    ("observed" if claim["assessment"] == "observed" else "not-reproduced"),
                    f"{cid}: observed outcome conflicts with review assessment")
    return review_data

def canonical(doc):
    return (json.dumps(doc, indent=2, ensure_ascii=False) + "\n").encode("utf-8")

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--root", type=Path, default=ROOT)
    mode = p.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--check", action="store_true")
    a = p.parse_args()
    doc = validate(a.root)
    content = canonical(doc)
    path = a.root / EXPORT
    if a.write:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
    else:
        require(path.read_bytes() == content, "Review export is stale or altered")
    print(f"OK: {len(doc['claims'])} independently referenced effect-review claims (unverified imports stay unverified)")

if __name__ == "__main__":
    main()
