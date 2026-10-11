#!/usr/bin/env python3
"""Measure *candidate* PS2 game coverage gained from complete bulk sources.

This only compares original authors' game-name strings. It does not establish
distinct released games, emulator build identity or working gameplay effects.
"""
import argparse
import json
import re
import unicodedata
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/"generated/v1/reports/ps2-bulk-source-coverage.json"
ROOT670="root670-cheatdevice-ps2"
ZIP="israpps-cheatdevice-ps2-extras"
PNACH="emucorex-cheatdevice-pnach"
QUOTED=re.compile(r'^"([^"]+)"\s*$')

def rows(source):
    return json.loads((ROOT/"sources"/(source+".json")).read_text())["files"]

def read(row):
    return (ROOT/row["archive_path"]).read_bytes().decode("utf-8","replace")

def quoted_names(text):
    return [x.group(1).strip() for line in text.splitlines()
            if (x:=QUOTED.match(line.strip())) and x.group(1).strip()]

def normalize(name,drop_region=False):
    name=unicodedata.normalize("NFKD",name).casefold()
    name="".join(ch for ch in name if not unicodedata.combining(ch))
    name=re.sub(r'\s*\((?:scus|sles|slus|slps|sces|scps|slpm)[-_0-9a-z.]+\)\s*$','',name)
    if drop_region:
        name=re.sub(r'(?:\s|\(|\[|[-_/])(?:ntsc|pal|usa|eur|europe|us|japan|jp)(?:\s|\)|\]|$)',' ',name)
    return re.sub(r'[^a-z0-9]+','',name)

def metric(root_names,ntsc,pal,pnach):
    base={normalize(n) for n in root_names if normalize(n)}
    combined=[*ntsc,*pal]
    variants={normalize(n) for n in combined if normalize(n)}
    relaxed_base={normalize(n,True) for n in root_names if normalize(n,True)}
    relaxed_all={normalize(n,True) for n in combined if normalize(n,True)}
    pnach_ids={normalize(n) for n in pnach if normalize(n)}
    return {
        "root670_distinct_normalized_source_labels":len(base),
        "zip_ntsc_distinct_normalized_source_labels":len({normalize(n) for n in ntsc}),
        "zip_pal_distinct_normalized_source_labels":len({normalize(n) for n in pal}),
        "zip_ntsc_pal_distinct_normalized_source_labels":len(variants),
        "zip_labels_matching_root670_strict":len(base & variants),
        "zip_labels_unmatched_to_root670_strict":len(variants - base),
        "zip_labels_unmatched_to_root670_after_region_relaxation":len(relaxed_all-relaxed_base),
        "pnach_distinct_source_game_labels":len(pnach_ids),
        "pnach_labels_matching_root670_strict":len(pnach_ids & base),
        "pnach_labels_unmatched_to_root670_strict":len(pnach_ids - base),
    }

def build():
    root_rows=rows(ROOT670)
    source=next(x for x in root_rows if x["upstream_path"]=="CheatDatabase.txt")
    baseline=quoted_names(read(source))
    if len(baseline)!=1483:raise ValueError(f"Root670 source sections changed: {len(baseline)}")
    info=rows(ZIP)
    packs={"NTSC_CHEATS.zip":[],"PAL_CHEATS.zip":[],"MASTERCODES.zip":[],"WIDESCREEN.zip":[]}
    filename_only={"NTSC_CHEATS.zip":0,"PAL_CHEATS.zip":0,"MASTERCODES.zip":0}
    native_counts={"NTSC_CHEATS.zip":0,"PAL_CHEATS.zip":0,"MASTERCODES.zip":0,"WIDESCREEN.zip":0}
    for row in info:
        upstream=row["upstream_path"]
        if not upstream.startswith("zip-members/"):continue
        _,pack,path=upstream.split("/",2)
        if pack not in packs:raise ValueError(f"Unknown ZIP pack {pack}")
        suffix=Path(path).suffix.lower()
        if suffix not in (".cht",".txt"):continue
        native_counts[pack]+=1
        label=quoted_names(read(row))
        if not label:
            label=[Path(path).stem.replace("_"," ")]
            if pack in filename_only:filename_only[pack]+=1
        packs[pack].extend(label)
    expected={"NTSC_CHEATS.zip":1880,"PAL_CHEATS.zip":1349,"MASTERCODES.zip":4013,"WIDESCREEN.zip":1}
    if native_counts!=expected:raise ValueError(f"ZIP native formats changed: {native_counts}")
    converted=rows(PNACH)
    src_pnach=[x for x in converted if x["upstream_path"].endswith(".pnach")]
    if len(src_pnach)!=473:raise ValueError(f"PNACH inventory changed: {len(src_pnach)}")
    pnach_names=[]
    for item in src_pnach:
        text=read(item)
        found=next((line[len("gametitle="):].strip() for line in text.splitlines()
                    if line.startswith("gametitle=")),None)
        if not found:found=Path(item["upstream_path"]).stem
        pnach_names.append(found)
    name_metrics=metric(baseline,packs["NTSC_CHEATS.zip"],packs["PAL_CHEATS.zip"],pnach_names)
    return {
        "schema_version":1,
        "interpretation":"source_name_candidates_only_not_verified_unique_games_or_execution",
        "matching":"simple casefolded letter/digit comparison; optional terminal region-label relaxation; heuristic and collision-prone",
        "source_original_game_sections":{"root670":len(baseline),
           "ntsc_cheat_source_files":native_counts["NTSC_CHEATS.zip"],
           "pal_cheat_source_files":native_counts["PAL_CHEATS.zip"],
           "mastercode_source_files":native_counts["MASTERCODES.zip"],
           "widescreen_source_files":native_counts["WIDESCREEN.zip"],
           "widescreen_source_game_sections":len(packs["WIDESCREEN.zip"]),
           "pnach_source_files":len(src_pnach)},
        "headerless_files_named_from_source_filename":filename_only,
        "source_name_comparison":name_metrics,
        "caveats":["Raw source code-line counts and executable cheat effect counts are different units.",
                   "The PNACH conversion derives from already archived CheatDevice raw cheats, so code occurrence growth is not code-design novelty.",
                   "Textual title matching cannot verify distinct releases, game regions, serial/CRC hashes or runnable codes.",
                   "Mastercodes enable other codes and are not counted as independent gameplay effects."],
        "source_revisions":{ROOT670:"7d9c525a3a8377ff0b8fe782ea692ed0f245b92c",
                            ZIP:"77b453ad41dace65789ca4c152c94b19f7f55934",
                            PNACH:"e1803c3bab8352d3e032194efeca03aaa489b8a5"},
    }

def main():
    ap=argparse.ArgumentParser(description=__doc__)
    mode=ap.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write",action="store_true")
    mode.add_argument("--check",action="store_true")
    args=ap.parse_args()
    data=build()
    serialized=json.dumps(data,indent=2,ensure_ascii=False,sort_keys=True)+"\n"
    if args.check:
        if not OUT.exists() or OUT.read_text()!=serialized:
            raise SystemExit("PS2 bulk-coverage report stale; run --write")
    else:
        OUT.parent.mkdir(parents=True,exist_ok=True)
        OUT.write_text(serialized)
    print(json.dumps({"result":"valid" if args.check else "written",
                      "metrics":data["source_name_comparison"],
                      "headerless":data["headerless_files_named_from_source_filename"]}))
if __name__=="__main__":main()
