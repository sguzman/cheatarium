#!/usr/bin/env python3
"""Publish pinned rights-unresolved filename inventories as metadata-only bundles.

No upstream cheat or patch body is fetched, decoded, or redistributed.
"""
import argparse
import gzip
import io
import json
from collections import defaultdict
from pathlib import Path

from external_bulk import ROOT, load, classify, source_link

OUT = Path("generated/v1")
FORMAT = "cheatarium-external-references-v1"
STATUS = "link_only_rights_review"


def packed(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def compressed(raw):
    buffer = io.BytesIO()
    with gzip.GzipFile(filename="", mode="wb", fileobj=buffer, mtime=0, compresslevel=9) as f:
        f.write(raw)
    return buffer.getvalue()


def render(source_root=ROOT):
    indexed = load(source_root)
    groups = defaultdict(list)
    for src in indexed:
        assert src["rights_status"] == STATUS
        for path, blob, size in src["files"]:
            groups[src["platform"]].append({
                "source_id": src["id"],
                "kind": classify(src["id"], path),
                "path": path,
                "git_blob_sha": blob,
                "bytes": size,
                "repository": src["repository"],
                "revision": src["snapshot_commit"],
                "upstream_url": source_link(src, path),
                "rights_status": STATUS,
            })
    files = {}
    catalog_entries = []
    for platform in sorted(groups):
        records = sorted(groups[platform], key=lambda r: (r["source_id"], r["path"]))
        # Never mix external links into code-bearing Cheatarium source bundles.
        doc = {
            "schema_version": 1, "format": FORMAT,
            "platform": platform, "metadata_only": True,
            "rights_status": STATUS, "gameplay_verified": False,
            "records": records,
        }
        rel = f"external/{platform}.json.gz"
        files[rel] = compressed(packed(doc))
        catalog_entries.append({
            "platform": platform,
            "artifact": rel,
            "source_files": len(records),
            "source_ids": sorted({r["source_id"] for r in records}),
        })
    catalog = {
        "schema_version": 1, "format": FORMAT,
        "metadata_only": True,
        "rights_status": STATUS, "gameplay_verified": False,
        "archive_included": False,
        "reference_files": sum(len(v) for v in groups.values()),
        "platforms": catalog_entries,
    }
    if catalog["reference_files"] != 17285 or len(catalog_entries) != 12:
        raise ValueError("Pinned external bulk inventory coverage changed")
    files["external/catalog.json"] = packed(catalog)
    return files


def main():
    p = argparse.ArgumentParser(description=__doc__)
    g = p.add_mutually_exclusive_group(required=True)
    g.add_argument("--write", action="store_true")
    g.add_argument("--check", action="store_true")
    args = p.parse_args()
    artifacts = render()
    for rel, raw in sorted(artifacts.items()):
        path = ROOT / OUT / rel
        if args.write:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(raw)
        elif not path.exists() or path.read_bytes() != raw:
            raise SystemExit("Stale or modified metadata-only external artifact: " + rel)
    print(f"OK: {len(artifacts)} deterministic external reference artifacts; 17285 link-only records")


if __name__ == "__main__":
    main()
