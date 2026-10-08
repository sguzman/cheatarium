#!/usr/bin/env python3
"""Independently compare SNES publication witnesses with intact .cht archive bytes.

A generated source bundle or a historical website alone cannot establish the
integrity of the original imported file. This audit verifies original Git-blob
SHA-1 and exact code fields before treating a listing as text evidence.
"""
import hashlib
import re
from pathlib import PurePosixPath


CODE_FIELD = re.compile(r"^cheat([0-9]+)_code\s*=\s*(.*)$")


def archive_path(root, relative):
    if not isinstance(relative, str) or "\\" in relative:
        raise ValueError("Unsupported original archive path")
    path = PurePosixPath(relative)
    if (path.is_absolute()
            or any(part in (".", "..") for part in relative.split("/"))
            or path.parts[:3] != ("archive", "libretro", "cht")
            or len(path.parts) < 4
            or path.suffix != ".cht"):
        raise ValueError("Unsafe or unexpected original archive path")
    candidate = root.joinpath(*path.parts)
    if not candidate.resolve().is_relative_to(root.resolve()):
        raise ValueError("Original archive path escapes repository root")
    return candidate


def parse_code_fields(raw):
    text = raw.decode("utf-8")
    codes = {}
    for line in text.splitlines():
        m = CODE_FIELD.fullmatch(line.strip())
        if m is None:
            continue
        ordinal = int(m.group(1))
        code = m.group(2).strip()
        if len(code) >= 2 and code.startswith('"') and code.endswith('"'):
            code = code[1:-1]
        if ordinal in codes:
            raise ValueError(f"Duplicate original cheat code ordinal {ordinal}")
        codes[ordinal] = code
    return codes


def audit_original_archives(root, registry, bundle):
    sources = {r["id"]: r for r in bundle["records"]}
    if len(sources) != len(bundle["records"]):
        raise ValueError("Duplicate indexed original source identifiers")
    grouped = {}
    for witness in registry["records"]:
        source_id = witness["source_record_id"]
        record = sources.get(source_id)
        if record is None:
            raise ValueError(f"Publication source is absent: {source_id}")
        grouped.setdefault(source_id, []).append(witness)

    checked = 0
    for source_id, witnesses in sorted(grouped.items()):
        record = sources[source_id]
        provenance = record["provenance"]
        if (source_id != provenance["source_id"] + ":" + provenance["upstream_path"]):
            raise ValueError(f"Index source ID disagrees with provenance: {source_id}")
        path = archive_path(root, provenance["archive_path"])
        raw = path.read_bytes()
        git_sha = hashlib.sha1(f"blob {len(raw)}".encode() + bytes([0]) + raw).hexdigest()
        if git_sha != provenance["git_blob_sha"]:
            raise ValueError(f"Original archive Git blob changed: {source_id}")
        original_codes = parse_code_fields(raw)
        for witness in witnesses:
            if witness["source_git_blob_sha"] != git_sha:
                raise ValueError(f"Publication witness uses another original blob: {source_id}")
            if original_codes.get(witness["source_ordinal"]) != witness["raw_code"]:
                raise ValueError(
                    f"Original .cht code/ordinal differs from publication witness: "
                    f"{source_id} #{witness['source_ordinal']}"
                )
            checked += 1
    return {"original_archived_files_checked": len(grouped),
            "publication_witness_ordinals_checked": checked}
