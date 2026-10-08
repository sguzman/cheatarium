#!/usr/bin/env python3
"""Deterministically prioritize unresolved SNES source '+' joins.

No code execution, ROM inspection, semantic guesses, or requests to other
repositories. Counts are original source-record occurrences, not distinct
working cheats or verified game releases.
"""
import argparse
import gzip
import json
from collections import defaultdict
from pathlib import Path

from validate_snes_publications import validate_registry

ROOT = Path(__file__).resolve().parents[1]
REPORT = Path("generated/v1/reports/snes-composition-review-queue.json")
TOP = 100
SAMPLES = 3


def calculate(bundle, documented, publications=None):
    if bundle.get("schema_version") != 1 or bundle.get("platform") != "snes":
        raise ValueError("Review queue requires SNES v1 source index")
    documented_set={(x["source_record_id"], x["source_ordinal"])
                    for x in documented["records"]}
    published_set = set()
    if publications is not None:
        validate_registry(publications, bundle)
        published_set = {(x["source_record_id"], x["source_ordinal"])
                         for x in publications["records"]}
    buckets=defaultdict(lambda: {
        "titles":set(), "source_records":set(), "regions":set(),
        "device_labels":set(), "occurrences":[], "publication_witnesses":set(),
    })
    total=0
    for record in bundle["records"]:
        key=record["candidate_game_key"] or f'unresolved:{record["id"]}'
        for cheat in record["codes"]:
            code=cheat.get("code") or ""
            if cheat.get("role")!="code" or "+" not in code:
                continue
            comp=cheat.get("composition")
            if (not isinstance(comp,dict)
                    or comp.get("relation") not in ("unresolved","revision-alternatives")):
                raise ValueError("Unclassified source join found in review queue input")
            ref=(record["id"], cheat["ordinal"])
            if comp["relation"]=="revision-alternatives":
                if ref not in documented_set:
                    raise ValueError("Undocumented version alternative")
                continue
            if ref in documented_set:
                raise ValueError("Reviewed source downgraded to unresolved")
            if comp.get("evidence") or comp.get("alternatives"):
                raise ValueError("Unresolved source must not have synthetic evidence")
            b=buckets[key]
            if ref in published_set:
                b["publication_witnesses"].add(ref)
            b["titles"].add(record["title_hint"])
            b["source_records"].add(record["id"])
            if record.get("region_hint"):
                b["regions"].add(record["region_hint"])
            if record.get("format_hint"):
                b["device_labels"].add(record["format_hint"])
            b["occurrences"].append({
                "source_record_id": record["id"],
                "ordinal": cheat["ordinal"],
                "source_code": code,
                "source_git_blob_sha": record["provenance"]["git_blob_sha"],
            })
            total+=1
    out=[]
    for key, b in buckets.items():
        b["occurrences"].sort(key=lambda c: (c["source_record_id"], c["ordinal"]))
        out.append({
            "candidate_game_key":key,
            "title_hints":sorted(b["titles"])[:10],
            "unresolved_source_occurrences":len(b["occurrences"]),
            "distinct_source_records":len(b["source_records"]),
            "historical_publication_witnesses":len(b["publication_witnesses"]),
            "occurrences_without_publication_witness":(
                len(b["occurrences"])-len(b["publication_witnesses"])),
            "region_hints":sorted(b["regions"]),
            "declared_device_hints":sorted(b["device_labels"]),
            "sample_originals":b["occurrences"][:SAMPLES],
        })
    out.sort(key=lambda b: (-b["unresolved_source_occurrences"], b["candidate_game_key"]))
    return {
        "schema_version":1,
        "platform":"snes",
        "scope":"unresolved-source-code-plus-semantics",
        "identity_confidence":"filename-derived-candidates-only",
        "effect_or_compatibility_verified":False,
        "rank_rule":"descending-unresolved-source-occurrences-then-ascending-candidate-key",
        "total_unresolved_source_occurrences":total,
        "total_historical_publication_witnesses":sum(
            len(b["publication_witnesses"]) for b in buckets.values()),
        "total_without_publication_witness":(
            total-sum(len(b["publication_witnesses"]) for b in buckets.values())),
        "candidate_groups_with_unresolved_joins":len(out),
        "published_top_limit":TOP,
        "top_groups":out[:TOP],
    }


def build(root):
    with gzip.open(root/"generated/v1/snes.json.gz","rt",encoding="utf-8") as f:
        bundle=json.load(f)
    doc=json.loads((root/"generated/v1/interpretations/snes.json").read_text(encoding="utf-8"))
    publications=json.loads((root/"interpretations/v1/snes-published-groups.json")
                            .read_text(encoding="utf-8"))
    data=calculate(bundle,doc,publications)
    audit=json.loads((root/"generated/v1/reports/snes-codec-coverage.json").read_text(encoding="utf-8"))
    if data["total_unresolved_source_occurrences"] != audit["counts"]["unresolved_plus_groups"]:
        raise ValueError("Review queue differs from SNES composition audit count")
    return data


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    mode=parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write",action="store_true")
    mode.add_argument("--check",action="store_true")
    parser.add_argument("--root",type=Path,default=ROOT)
    args=parser.parse_args()
    output=args.root/REPORT
    data=build(args.root)
    contents=(json.dumps(data,indent=2,ensure_ascii=False)+"\n").encode("utf-8")
    if args.write:
        output.parent.mkdir(parents=True,exist_ok=True)
        output.write_bytes(contents)
    elif output.read_bytes()!=contents:
        raise ValueError("SNES source composition review queue is stale or altered")
    print("OK: SNES source composition review queue:",
          data["total_unresolved_source_occurrences"],"unresolved joins,",
          data["candidate_groups_with_unresolved_joins"],"candidate buckets")


if __name__=="__main__":
    main()
