#!/usr/bin/env python3
"""Audit ALL four pinned CheatDevicePS2 extra-cheat ZIPs before bulk import.

This is source-format discovery only. It does not select individual titles,
mutate collections, treat source names as verified games or execute cheats.
"""
from collections import Counter
from pathlib import Path
import json
import subprocess
import tempfile
import zipfile

REPOSITORY = "https://github.com/israpps/CheatDevicePS2.git"
REVISION = "77b453ad41dace65789ca4c152c94b19f7f55934"
ZIPS = ("MASTERCODES.zip", "NTSC_CHEATS.zip", "PAL_CHEATS.zip", "WIDESCREEN.zip")

def main():
    with tempfile.TemporaryDirectory(prefix="cheatarium-cheatdevice-archives-") as temp:
        src = Path(temp) / "CheatDevicePS2"
        subprocess.run(["git","clone","--quiet","--filter=blob:none","--depth=1",
                        REPOSITORY,str(src)],check=True)
        subprocess.run(["git","-C",str(src),"fetch","--quiet","--depth=1",
                        "origin",REVISION],check=True)
        subprocess.run(["git","-C",str(src),"checkout","--quiet","--detach",REVISION],
                       check=True)
        actual=subprocess.check_output(["git","-C",str(src),"rev-parse","HEAD"],
                                       text=True).strip()
        if actual!=REVISION:
            raise RuntimeError("Pinned archive revision mismatch")
        reports=[]
        for filename in ZIPS:
            path=src/"extra_cheats"/filename
            with zipfile.ZipFile(path) as archive:
                entries=[i for i in archive.infolist() if not i.is_dir()]
                if not entries: raise RuntimeError(f"Empty archive {filename}")
                if any(i.flag_bits & 0x1 for i in entries):
                    raise RuntimeError(f"Encrypted source archive: {filename}")
                ext=Counter(Path(i.filename).suffix.lower() for i in entries)
                top=Counter(i.filename.split("/")[0] for i in entries)
                report={"zip":filename,"source_bytes":path.stat().st_size,
                        "entries":len(entries),"uncompressed_bytes":sum(i.file_size for i in entries),
                        "extensions":dict(ext),"sample_paths":[x.filename for x in entries[:12]],
                        "top_folders":top.most_common(12)}
                reports.append(report)
                print(json.dumps(report,ensure_ascii=False))
                for sample in entries[:2]:
                    if sample.filename.lower().endswith((".cht",".txt")):
                        raw=archive.read(sample)
                        print("FORMAT SAMPLE",json.dumps({"archive":filename,
                            "member":sample.filename,"bytes":len(raw),
                            "first_lines":raw.decode("utf-8","replace").splitlines()[:26]},
                            ensure_ascii=False))
        print("AUDIT TOTAL:",json.dumps({"archives":len(ZIPS),
             "members":sum(x["entries"] for x in reports),
             "uncompressed_bytes":sum(x["uncompressed_bytes"] for x in reports)}))
if __name__=="__main__":
    main()
