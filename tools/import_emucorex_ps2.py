#!/usr/bin/env python3
"""Import the entire pinned GPL-derived CheatDevice-to-PCSX2 PNACH collection."""
import argparse,hashlib,json,shutil,subprocess,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
ID="emucorex-cheatdevice-pnach"
UPSTREAM="https://github.com/sashkinbro/EmuCoreX-Cheat"
REV="e1803c3bab8352d3e032194efeca03aaa489b8a5"
PREFIX="files/root670-cheatdevice/"
NOTICES=("README.md","sources.json","LICENSES/GPL-3.0.txt")
MANIFEST=ROOT/"sources"/f"{ID}.json"

def sha(raw):
    return hashlib.sha1(f"blob {len(raw)}\0".encode()+raw).hexdigest()

def acquire(folder):
    actual=subprocess.check_output(["git","-C",str(folder),"rev-parse","HEAD"],text=True).strip()
    if actual!=REV: raise RuntimeError(f"Wrong source revision: {actual}")
    allpaths=subprocess.check_output(["git","-C",str(folder),"ls-tree","-r","--name-only","HEAD"],text=True).splitlines()
    files=sorted(p for p in allpaths if p.startswith(PREFIX))
    if len(files)!=473 or any(not p.endswith(".pnach") for p in files):
        raise RuntimeError(f"Expected 473 original PCSX2 PNACHs, got {len(files)}")
    old={}
    if MANIFEST.exists():
        prior=json.loads(MANIFEST.read_text())
        if prior.get("snapshot_commit")!=REV: raise RuntimeError("Revision conflict")
        old={x["upstream_path"]:x for x in prior["files"]}
    new=0; rows=[]; missing_markers=0
    for path in sorted([*files,*NOTICES]):
        origin=folder/path
        if not origin.is_file() or origin.is_symlink(): raise RuntimeError(f"Missing original: {path}")
        raw=origin.read_bytes()
        if not raw: raise RuntimeError(f"Empty original: {path}")
        if path.endswith(".pnach"):
            contents=raw.decode("utf-8")
            if "patch=" not in contents: raise RuntimeError(f"Missing native patch: {path}")
            if "SPDX-License-Identifier: GPL-3.0-only" not in contents: missing_markers+=1
        record={"upstream_path":path,"archive_path":f"archive/{ID}/{path}","git_blob_sha":sha(raw)}
        if path in old and old[path]!=record: raise RuntimeError(f"Original changed: {path}")
        dest=ROOT/record["archive_path"]
        if dest.exists():
            if dest.read_bytes()!=raw: raise RuntimeError(f"Archive tampered: {path}")
        else:
            dest.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(origin,dest);new+=1
        rows.append(record)
    if set(old)-{x["upstream_path"] for x in rows}:raise RuntimeError("Dropped existing originals")
    data={"schema_version":1,"id":ID,"repository":UPSTREAM,"snapshot_commit":REV,
          "license":"GPL-3.0-only (derived from root670 CheatDevice; historic code writers' rights unresolved)",
          "archive_prefix":f"archive/{ID}/",
          "scope":"All 473 distinct serial-CRC-labeled PNACH originals from the complete CheatDevice conversion folder",
          "rights_note":"The original converter output identifies GPL-derived CheatDevice code authors; the GPL notice and sources metadata are preserved. Some historical contributors may have independent rights. Game serial/CRC mapping and operation unverified by Cheatarium; substantial overlap with the already archived raw root670 dataset.",
          "source_pnach_files":len(files),"missing_embedded_spdx_markers":missing_markers,"files":rows}
    MANIFEST.write_text(json.dumps(data,indent=2,ensure_ascii=False)+"\n")
    print(json.dumps({"revision":REV,"native_PNACH_files":len(files),"new_files_including_notices":new,
                      "missing_embedded_SPDX_markers":missing_markers,"inventory_records":len(rows)},indent=2))

def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--checked-out-upstream",type=Path)
    args=ap.parse_args()
    if args.checked_out_upstream:return acquire(args.checked_out_upstream)
    with tempfile.TemporaryDirectory(prefix="cheatarium-pnach-bulk-") as tmp:
        src=Path(tmp)/"EmuCoreX-Cheat"
        subprocess.run(["git","clone","--quiet","--filter=blob:none","--depth=1","--sparse",UPSTREAM+".git",str(src)],check=True)
        subprocess.run(["git","-C",str(src),"fetch","--quiet","--depth=1","origin",REV],check=True)
        subprocess.run(["git","-C",str(src),"checkout","--quiet","--detach",REV],check=True)
        subprocess.run(["git","-C",str(src),"sparse-checkout","set","--cone","files/root670-cheatdevice","LICENSES"],check=True)
        acquire(src)
if __name__=="__main__":main()
