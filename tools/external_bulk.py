#!/usr/bin/env python3
"""Reproducible metadata-only inventory for rights-unresolved bulk cheat sources.

The pinned references expose names, Git blob IDs, and byte sizes. This tool
never copies third-party cheat or patch bodies into the Cheatarium archive.
"""
import argparse
import json
import re
import sys
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FOLDER = Path("sources/external-bulk")
SOURCES = {
    "leumas-gamecube-wiird-zips": {
        "repository": "LeumasPlays/WiiRD-GameCube-Cheats",
        "commit": "102502ee0dccd733b389f27ab073315d1033ec27",
        "platform": "gamecube", "expected": {"cheat": 3},
        "rights_note": "Regional GameCube/WiiRD ZIP compilations sourced by LeumasPlays from gc-forever community forum authors. No repository-wide LICENSE at pinned revision. ZIP bodies, cheat codes and per-game internal members are excluded; not tested or verified by the compiler except a handful of NTSC-U codes. The ZIP files are three containers, not three game titles or effects.",
    },
    "katsumaki-wiiu-xml": {
        "repository": "KatsuMaki1/Wiiu-Cheats-Repo",
        "commit": "8de6dab4613fb57b0a3575bfa93c3b4c1ea1cbbf",
        "platform": "wii-u", "expected": {"cheat": 9},
        "rights_note": "Nine source-native XML cheat documents from a Wii U/Cemu compilation with individual code authors and community/Discord/GBAtemp sources credited. No root LICENSE at pinned revision and no grant from each code author. Excludes all Cemu SDK binaries and third-party .NET documentation.",
    },
    "ucodes-wiiu-json": {
        "repository": "vincent-coding/uCodes",
        "commit": "2dfb558ae387837fbecaefda7a3cb8b801d03ef7",
        "platform": "wii-u", "expected": {"cheat": 4},
        "rights_note": "Four Wii U JSON cheat collections by uCodes, including codes with per-entry third-party author credits. Root MIT license is held by website author, but the presence of author credits does not establish that third-party cheat bodies are licensed under MIT. One exact-blob duplicate JSON file. No code texts imported.",
    },
    "duckstation-chtdb": {
        "repository": "duckstation/chtdb",
        "commit": "e3cce509936261808b971199be3a20e84d0b9482",
        "platform": "ps1", "expected": {"cheat": 4497, "patch": 138},
        "rights_note": "The upstream LICENSE explicitly reserves ownership of cheats and patches to original authors; only processing scripts receive its MIT grant. File paths, hashes and byte sizes are indexed here, not code or patch bodies.",
    },
    "crashgg-ps1-chtdb-delta": {
        "repository": "crashGG/ps1.chtdb",
        "commit": "a25a878aac35b3c4c4c9aab43ec266b900259b44",
        "platform": "ps1", "expected": {"cheat": 263, "patch": 5},
        "rights_note": "Pinned derivative DuckStation PS1 database: 4,485 original .cht file occurrences, of which 4,217 exact Git blobs are already indexed under duckstation-chtdb; retain only 268 different-byte paths (263 cheats and 5 patches). The upstream LICENSE expressly reserves cheat and patch copyright to original authors and grants MIT only for processing scripts. These different bytes can be older/modified versions of the SAME game and are not new code effects, verified builds or redistributable source contents.",
    },
    "deadskullz-nds-formats": {
        "repository": "szTheory/NDS-Cheat-Databases",
        "commit": "0173af14d33e2c045e7c5e30c71d369624a33020",
        "platform": "ds", "expected": {"cheat": 9},
        "rights_note": "Community-converted Nintendo DS Action Replay collections credited to DeadSkullzJr, distributed in nine native flashcart/emulator file formats (eight byte-distinct Git blobs because cheat.db and cheat_EN.db are exact copies). The mirror includes an AGPL-3.0 LICENSE and links to original GBAtemp/Bitbucket creators, but its root notice does not establish permission from every original code author. Whole binary/XML/7z cheat databases are link-only; no binary body or verified title/effect identities copied.",
    },
    "saramagrean-psp-cwcheat": {
        "repository": "Saramagrean/CWCheat-Database-Plus-",
        "commit": "5ab71718f01810e7980d29a48b4755463233c55b",
        "platform": "psp", "expected": {"cheat": 1},
        "rights_note": "An 8.9 MB PSP/PPSSPP CWCheat cheat.db aggregation from Saramagrean; its README explicitly credits third-party PPSSPP forums, GameHacking, LunaMoo, Kabuto_Kun, Raing3 and others. No root LICENSE found at pinned revision; the credited creators' code redistribution rights have not been confirmed. This is a binary database reference, not validated individual cheats or author permission.",
    },
    "shadowninja-ps2-pnach-delta": {
        "repository": "shadowninja826/pcsx2_pnach_cheats",
        "commit": "56d328be4c4f048896788f8203703b822656b768",
        "platform": "ps2", "expected": {"cheat": 48},
        "rights_note": "Pinned PS2 PNACH mirror: 628 original .pnach file paths, of which 579 exactly match the pinned xs1l3n7x-pcsx2 Git blobs. Exclude those exact mirrors and one additional empty (0-byte) .pnach; retain 48 byte-distinct nonempty original file paths as link-only author-rights leads. No root LICENSE in the pinned tree; README only describes PNACH files for PS2. Different bytes/CRC labels do not establish distinct games, valid effects, source ownership, or permission to redistribute.",
    },
    "libretro-mame2010-cheat-archive": {
        "repository": "libretro/mame2010-libretro",
        "commit": "dff8aadd1c3f38215af3955746d6e19abe0ddcea",
        "platform": "arcade-mame", "expected": {"cheat": 1},
        "rights_note": "Complete upstream MAME 2010 cheat ZIP used by the libretro core. Exact identical Git blob to galisteogames/mame2010_cheat cheat.zip; do NOT count that mirror as another source. Archive members and working cheats are NOT enumerated, and Pugsy/other original code-author rights are not established by an emulator source LICENSE. Pinned filename, bytes and SHA only.",
    },
    "libretro-mame2003plus-cheat-archive": {
        "repository": "libretro/mame2003-plus-libretro",
        "commit": "aa2b32f913ce40daf373c80ad050ff172208005c",
        "platform": "arcade-mame", "expected": {"cheat": 1},
        "rights_note": "Historic MAME 2003-Plus libretro metadata/cheat.dat compiled cheat database. Project's classic MAME license covers emulator source subject to restrictions but does NOT establish contributor rights to all compiled historical cheats. One 7.81 MB source file, not an enumerated or verified game/cheat inventory; link-only.",
    },
    "smoker1-fba-official-ini": {
        "repository": "Smoker1/RetroArch-174-and-above-Cheats",
        "commit": "d07b731562d3a68d089dc0a1ba62311b65a1d312",
        "platform": "arcade-fba", "expected": {"cheat": 98},
        "rights_note": "Smoker1's pinned RetroArch 1.7.4+ source contains 98 source-native FB Alpha 'Official Cheats' INI file names, but only 45 different Git blob SHA-1s (many exact-byte clone/variant aliases). The repository README calls these FB Alpha cheats and explains manual address adaptation to RetroArch; it contains no grant of redistribution rights from the original FBA cheat authors. These are FBA-native cheat files, NOT automatically compatible with FinalBurn Neo or MAME. Keep source paths, byte sizes and Git IDs only; effects, game identities and license unresolved.",
    },
    "pacnpal-mupen-n64-cheatfile": {
        "repository": "pacnpal/mupen-cheats",
        "commit": "3dd74ad738741979aaf5ac5fda2611fe0ad5c53a",
        "platform": "n64", "expected": {"cheat": 1},
        "rights_note": "One 1,281,051-byte Mupen64Plus-native mupencheat.txt Gameshark cheat database. The README credits new GoldenEye, Perfect Dark and Donkey Kong 64 fixes/code caves alongside preexisting distributed Mupen and late-1990s GameShark FAQ code sources. No root LICENSE file in pinned tree and no original-source author-wide permission established. One file is not one code or one game and neither compatibility nor execution is verified. Read-only pinned link and Git hash only.",
    },
    "xenia-canary-game-patches": {
        "repository": "xenia-canary/game-patches",
        "commit": "106bb0a40be27b5a2444162562975e4885e1ce74",
        "platform": "xbox-360", "expected": {"patch": 501},
        "rights_note": "No repository-wide LICENSE was present in the pinned tree; patch authorship and redistribution permissions must be reviewed. Patches are not necessarily gameplay cheats. Only file names, hashes and sizes are indexed.",
    },
    "pcsx2-official-patches": {
        "repository": "PCSX2/pcsx2_patches",
        "commit": "9f82a4d2b8a2aaf83807421f18d5d30263ea826c",
        "platform": "ps2", "expected": {"patch":4710},
        "rights_note": "The official PCSX2 repository aggregates patches written by many independent authors, and no blanket permission for their code was located at this pinned revision. Native PNACH patch files are link-only references, not imported.",
    },
    "r0ah-vitacheat": {
        "repository": "r0ah/vitacheat",
        "commit": "bb8158a1c696914a8ea2299889d42ab9a57a3ab2",
        "platform": "vita", "expected": {"cheat":677},
        "rights_note": "Community-submitted VitaCheat code authors retain separate rights; no root redistribution LICENSE was found at the pinned revision. This inventory includes 676 standard PSV files and one specially named PSV-NSFW variant; no payload is rehosted.",
    },
    "shumnot-vitacheat": {
        "repository": "ShumnoT/VitaCheatDatabase",
        "commit": "a394316be0fe7b233b49b7ba22f14fae56a1b9c8",
        "platform": "vita", "expected": {"cheat":119},
        "rights_note": "The database credits multiple earlier websites and authors and lacks a blanket source-code redistribution grant. Preserve only original filename, Git blob hash and byte count.",
    },
    "bucanero-dreamcast-cheats": {
        "repository": "bucanero/dreamcast-cheats",
        "commit": "13c7a5c9957331398252e2a8916be7dd61b68144",
        "platform": "dreamcast", "expected": {"cheat":387},
        "rights_note": "The GPL-3.0 repository-wide grant does not independently establish redistribution rights for third-party historical cheats copied or adapted from external authors. The original markdown cheat text remains link-only.",
    },
    "ademola-switch-native": {
        "repository": "ADEMOLA200/Switch-Emulator-Mod-Database",
        "commit": "abd55774c369b9c3a4df960e7afdb0391cb52056",
        "platform": "switch", "expected": {"cheat":1927,"patch":655},
        "rights_note": "The MIT-licensed upstream aggregates many separately authored Switch cheat/graphics mods and patch sources; original contributors' redistribution rights are unresolved. Links, paths and hashes only; FPS/resolution patches are not necessarily cheats.",
    },
    "ademola-3ds-native": {
        "repository": "ADEMOLA200/Switch-Emulator-Mod-Database",
        "commit": "abd55774c369b9c3a4df960e7afdb0391cb52056",
        "platform": "3ds", "expected": {"cheat":257},
        "rights_note": "The upstream README explicitly credits many independent modding projects and its root MIT license does not establish rights to the community-contributed 3DS title-ID cheats. Preserve only pinned 3DS title-ID file paths, byte sizes and original Git blob hashes; no cheat bodies copied.",
    },
    "rogamer97-asm-3ds": {
        "repository": "RoGamer97/ASM-Cheats-and-Patches",
        "commit": "8ec531942871cd4e0aaefe5eb596fb2b49e9ae92",
        "platform": "3ds", "expected": {"cheat":53},
        "rights_note": "No root repository LICENSE at pinned revision. Markdown/ASM/PCHTXT/IPS material has separate original authors; only links, original paths, blob hashes and sizes are indexed. No code bodies copied.",
    },
    "rogamer97-asm-ds": {
        "repository": "RoGamer97/ASM-Cheats-and-Patches",
        "commit": "8ec531942871cd4e0aaefe5eb596fb2b49e9ae92",
        "platform": "ds", "expected": {"cheat":5},
        "rights_note": "No root repository LICENSE at pinned revision. Markdown/ASM/PCHTXT/IPS material has separate original authors; only links, original paths, blob hashes and sizes are indexed. No code bodies copied.",
    },
    "rogamer97-asm-gamecube": {
        "repository": "RoGamer97/ASM-Cheats-and-Patches",
        "commit": "8ec531942871cd4e0aaefe5eb596fb2b49e9ae92",
        "platform": "gamecube", "expected": {"cheat":28,"patch":2},
        "rights_note": "No root repository LICENSE at pinned revision. Markdown/ASM/PCHTXT/IPS material has separate original authors; only links, original paths, blob hashes and sizes are indexed. No code bodies copied.",
    },
    "rogamer97-asm-wii": {
        "repository": "RoGamer97/ASM-Cheats-and-Patches",
        "commit": "8ec531942871cd4e0aaefe5eb596fb2b49e9ae92",
        "platform": "wii", "expected": {"cheat":305,"patch":1},
        "rights_note": "No root repository LICENSE at pinned revision. Markdown/ASM/PCHTXT/IPS material has separate original authors; only links, original paths, blob hashes and sizes are indexed. No code bodies copied.",
    },
    "rogamer97-asm-switch": {
        "repository": "RoGamer97/ASM-Cheats-and-Patches",
        "commit": "8ec531942871cd4e0aaefe5eb596fb2b49e9ae92",
        "platform": "switch", "expected": {"cheat":2,"patch":416},
        "rights_note": "No root repository LICENSE at pinned revision. Markdown/ASM/PCHTXT/IPS material has separate original authors; only links, original paths, blob hashes and sizes are indexed. No code bodies copied.",
    },
    "dospamu-rpcs3-artemis-patches": {
        "repository": "DoSpamu/RPCS3toArtemisPatches",
        "commit": "193ad2fc8001296796eb951f054efd855631ba47",
        "platform": "ps3", "expected": {"patch":2687},
        "rights_note": "MIT root repository contains RPCS3 and PSXPlace derived community patches by separate credited authors; publisher's MIT notice alone does not establish their redistribution rights. Original Artemis NCL conversions are link-only, may overlap existing Artemis codes, and upstream 'confirmed' labels are unverified by Cheatarium.",
    },
    "chidreams-rpcs3-patches": {
        "repository": "chidreams/Artemis-Patch-Collection-RPCS3",
        "commit": "c71ea69d28dc04ae8c7bc85cd513a8fa08909ad2",
        "platform": "ps3", "expected": {"patch":239},
        "rights_note": "MIT upstream collates historically sourced Artemis/RPCS3 patches from community authors; original-code rights are not independently confirmed. The 239 root-level named patch blobs have no file extensions; they are links only, not verified cheat effects.",
    },
    "apelfactory-ppsspp-cheatdb": {
        "repository": "ApelFactory/ppsspp-psp-cheatdb",
        "commit": "a0caf4c7e151e813781730046e46dabd815294e3",
        "platform": "psp", "expected": {"cheat":1},
        "rights_note": "The 8.7 MB PSP CWCheat cheat.db database has no root redistribution LICENSE at the pinned snapshot and contains historically compiled third-party cheat codes. Retain only its filename, byte size, original Git blob identity and pinned upstream URL.",
    },
    "real-f00-usrcheat-ds": {
        "repository": "real-F-00/usrcheat-mkds",
        "commit": "c0f4b13d9982add3e155ba99cd9cbeb9ab933f59",
        "platform": "ds", "expected": {"cheat":1},
        "rights_note": "The repository's Unlicense cannot by itself establish redistribution permission for separately authored cheat codes within its 42.98 MB binary usrcheat.dat DS database. No code, binary cheat payload, or game identities have been copied.",
    },
    "hamlet-switch-cheatdb": {
        "repository": "HamletDuFromage/switch-cheats-db",
        "commit": "9a062e9b9a9e5b422d0310cf5c829a3855cb77e7",
        "platform": "switch", "expected": {"cheat":4746, "patch":532},
        "rights_note": "The upstream README explicitly describes mirrors of the GBAtemp Switch cheat collection and ChanseyIsTheBest graphics/FPS cheat database, credits independent cheat authors, and provides no root redistribution LICENSE. Preserve only original JSON filenames, Git blob identities, sizes and links. Cheat/gfx mirrors may repeat content and are not independently verified game titles, cheat effects, or compatible builds.",
    },
    "ibnux-switch-cheatdb": {
        "repository": "ibnux/switch-cheat",
        "commit": "14f899df1cecca3c2359125b5b4876c2deeaab20",
        "platform": "switch", "expected": {"cheat":3136},
        "rights_note": "Upstream README identifies a compiled Switch cheat database assembled from LordApple SXOS-CheatDB, WerWolv EdiZon and other independent sources; no root LICENSE is present. Preserve only native title/build-ID cheat source filenames, pinned Git hashes and byte counts. No historical code bodies or game-build compatibility claims are redistributed.",
    },
    "sthetix-switch-mirror": {
        "repository": "sthetix/nx-cheats-db",
        "commit": "e5587ee6844bf9cb7fc6513a49cbf7385387b3bf",
        "platform": "switch", "expected": {"cheat":5329,"patch":532},
        "rights_note": "The upstream README explicitly credits external GBAtemp, ChanseyIsTheBest, HamletDuFromage and Cheatslips authors, and has no root redistribution license. Its JSON files are mirrored community cheat or graphics/60FPS sources: path and Git blob metadata only. Do not count mirrors as independent games, verified cheat effects, or permissions to copy code.",
    },
    "journeyover-3ds-ctrpf": {
        "repository": "JourneyOver/CTRPF-AR-CHEAT-CODES",
        "commit": "ca51e1de9a0d5b7ea0544da4477b1a11f720165e",
        "platform": "3ds", "expected": {"cheat":628},
        "rights_note": "The CTRPF AR collection accepts historically community-authored Nintendo 3DS cheat contributions and has no root LICENSE at this pinned revision. Reuse and attribution rights of individual original code authors remain unresolved; preserve title-ID source paths, Git blob hashes and byte sizes, not cheat codes.",
    },
    "daowace-pcsx2-consolidated": {
        "repository": "DAOWAce/pcsx2_cheats_collection",
        "commit": "a18b78b3b59ce0f96195606a1ecb6a38744948f5",
        "platform": "ps2", "expected": {"cheat": 5483},
        "rights_note": "Community-consolidated PCSX2 PNACH set deriving from Ultimate PCSX2 Cheater, root670 CheatDevice, a local legacy collection and public forks. No repository LICENSE provides a blanket grant for historical individual cheat authors. Pin original file paths and blob identities only; structural validation or claimed CRC matches do not prove gameplay execution or permit redistributing code.",
    },
    "xs1l3n7x-pcsx2": {
        "repository": "xs1l3n7x/pcsx2_cheats_collection",
        "commit": "6cb57b55939f47dd91a157bfb6cdc7fd157a780c",
        "platform": "ps2", "expected": {"cheat": 579},
        "rights_note": "Original PCSX2 cheat conversion project credits independent Ultimate PCSX2 Cheater and CheatDevice contributors, but the pinned tree has no LICENSE. Keep PNACH file paths, hashes and sizes as link-only references; conversions and CRC labels do not independently verify gameplay compatibility or code authorship.",
    },
    "ps2-widescreen-staging": {
        "repository": "PS2-Widescreen/pnach-DB",
        "commit": "91902dd63b67b7c8e2fcdf71b165b8e21f415346",
        "platform": "ps2", "expected": {"patch": 3632},
        "rights_note": "Upstream PS2-Widescreen's pnach-DB stages 3,632 source-native widescreen PNACH patches for PCSX2 and lacks a repository-wide LICENSE at the pinned commit. Patches may derive from numerous independent code authors; filenames/hashes/sizes only. Do not presume gameplay-cheat status, ROM/build compatibility, author permission, or redistribute original patch bodies.",
    },
}
# Filename-only, exact-blob remainder from two historical RetroArch forks.
# Folder labels remain evidence, not emulator compatibility claims.
MIRROR_GAP_GROUPS = {
    "gcmcom-legacy-gba": ("gcmcom-libretro-legacy", ("Nintendo - Game Boy Advance",), "gba", 2),
    "gcmcom-legacy-nes": ("gcmcom-libretro-legacy", ("Nintendo - Nintendo Entertainment System",), "nes", 8),
    "gcmcom-legacy-snes": ("gcmcom-libretro-legacy", ("Nintendo - Super Nintendo Entertainment System",), "snes", 1),
    "smoker1-legacy-fbneo": ("smoker1-retroarch-174", ("FinalBurn Neo",), "arcade-fbneo", 32),
    "smoker1-legacy-dreamcast": ("smoker1-retroarch-174", ("Reicast", "Sega - Dreamcast"), "dreamcast", 12),
    "smoker1-legacy-naomi": ("smoker1-retroarch-174", ("Reicast-NAOMI",), "arcade-naomi", 9),
    "smoker1-legacy-othercores": (
        "smoker1-retroarch-174",
        ("Citra", "Gambatte", "Mednafen NeoPop", "Mednafen VB", "PicoDrive",
         "PrBoom", "Sega - Saturn", "Snes9x 2010", "VBA Next", "ecwolf"),
        "multi-retroarch", 22,
    ),
}
MIRROR_ORIGINS = {
    "gcmcom-libretro-legacy": ("gcmcom/libretro-database", "4667ad6247352f57d44f2ad7b36f19f3e804aba9", 10872),
    "smoker1-retroarch-174": ("Smoker1/RetroArch-174-and-above-Cheats", "d07b731562d3a68d089dc0a1ba62311b65a1d312", 518),
}
MIRROR_RIGHTS = (
    "Source-native CHT file references filtered by exact Git-blob identity against three "
    "pinned Libretro, Emupedia and BitHeX source sets. This does not establish uniqueness "
    "versus every archived/external collection, game compatibility, functional cheat effects, "
    "original-author reuse rights, or any permission to copy cheat bodies. Metadata and links only."
)
for _id, (_origin, _folders, _platform, _count) in MIRROR_GAP_GROUPS.items():
    _repo, _commit, _total = MIRROR_ORIGINS[_origin]
    SOURCES[_id] = {
        "repository": _repo, "commit": _commit, "platform": _platform,
        "expected": {"cheat": _count}, "rights_note": MIRROR_RIGHTS,
    }


# Independently pinned source-level Git-blob inventories, partitioned by
# original console/format. These are searchable external links, never cheats.
SCOUT_GROUPS = {
    "eignar17-fba-ini": ("eignar17-arcade", "arcade-fba", 253),
    "eignar17-arcade-dat": ("eignar17-arcade", "arcade-kawaks", 368),
    "riyoway-nintendo-multiformat": ("riyoway-nintendo", "multi-nintendo", 2),
    "riyoway-3ds-json": ("riyoway-nintendo", "3ds", 1),
    "riyoway-nes-json": ("riyoway-nintendo", "nes", 1),
    "riyoway-switch-json": ("riyoway-nintendo", "switch", 1),
    "riyoway-wiiu-export": ("riyoway-nintendo", "wii-u", 2),
    "slade1972-segacd-zip": ("slade1972-segacd", "segacd", 51),
    "skillercmp-xemu-text": ("skillercmp-xemu", "xbox", 12),
}
SCOUT_MATCHERS = {
    "eignar17-fba-ini": r"Final Burn Alpha Cheat Archive/[^/]+\.ini",
    "eignar17-arcade-dat": r"cheats/[^/]+\.dat",
    "riyoway-nintendo-multiformat": r"(?:cheatdb\.db|exports/JSON/Nintendo/all\.jsonl)",
    "riyoway-3ds-json": r"exports/JSON/Nintendo/3ds\.json",
    "riyoway-nes-json": r"exports/JSON/Nintendo/nes\.json",
    "riyoway-switch-json": r"exports/JSON/Nintendo/switch\.json",
    "riyoway-wiiu-export": r"(?:exports/JSON/Nintendo/wiiu\.json|exports/JGeckoU/AUMJ88\.xml)",
    "slade1972-segacd-zip": r"Cheats/[^/]+\.zip",
    "skillercmp-xemu-text": r"Cheats/[^/]+\.txt",
}
from audit_bulk_source_scout import verify as verify_scout
_SCOUT_REPORT = json.loads((ROOT / "research/bulk-source-scout-v1.json").read_text("utf-8"))
verify_scout(_SCOUT_REPORT, ROOT)
_SCOUT_BY_ORIGIN = {s["id"]: s for s in _SCOUT_REPORT["sources"]}
for _id, (_origin, _platform, _count) in SCOUT_GROUPS.items():
    _scout = _SCOUT_BY_ORIGIN[_origin]
    SOURCES[_id] = {
        "repository": _scout["repository"].removeprefix("https://github.com/"),
        "commit": _scout["snapshot_commit"], "platform": _platform,
        "expected": {"cheat": _count},
        "rights_note": (_scout["rights_note"] +
            " Original body remains excluded; pinned source paths, Git blob SHA-1 "
            "identities and byte lengths are references only. Do not infer "
            "functional cheat effects, emulator compatibility or verified game builds."),
    }


def pinned_scout_rows(source):
    """Reconcile platform link-only partitions against the immutable scout."""
    report = json.loads((ROOT / "research/bulk-source-scout-v1.json").read_text("utf-8"))
    verify_scout(report, ROOT)
    origin, _platform, expected = SCOUT_GROUPS[source]
    parts = [s for s in report["sources"] if s["id"] == origin]
    if len(parts) != 1:
        raise ValueError("Missing source scout: " + source)
    rows = sorted((r for r in parts[0]["files"] if classify(source, r[0])),
                  key=lambda r: r[0])
    if len(rows) != expected:
        raise ValueError("Pinned scout partition changed: " + source)
    return rows


def pinned_mirror_gap_rows(source):
    """Exact file candidates from previously audited, immutable Git trees."""
    report = json.loads((ROOT / "research/retroarch-mirror-source-gaps-v1.json").read_text("utf-8"))
    if (report.get("format") != "cheatarium-retroarch-pinned-mirror-byte-gaps-v1"
            or report.get("schema_version") != 1
            or report.get("metadata_only") is not True
            or report.get("cheat_or_archive_bodies_included") is not False
            or report.get("rights_clearance_asserted") is not False
            or report.get("file_occurrences_not_matching_three_corpora") != 86):
        raise ValueError("Pinned RetroArch mirror audit changed")
    origin, folders, _, expected_count = MIRROR_GAP_GROUPS[source]
    records = [r for r in report["sources"] if r["id"] == origin]
    if len(records) != 1:
        raise ValueError("Missing pinned mirror source")
    upstream = records[0]
    _repo, commit, original_file_count = MIRROR_ORIGINS[origin]
    if (upstream["snapshot_commit"] != commit
            or upstream["original_cht_file_occurrences"] != original_file_count
            or upstream["rights_status"] != "link_only_rights_review"
            or upstream["redistribution_cleared"] is not False
            or upstream["code_payloads_included"] is not False):
        raise ValueError("Pinned mirror provenance or rights changed")
    rows = sorted(
        [[r["source_path"], r["git_blob_sha"], r["bytes"]]
         for r in upstream["unmatched_source_file_metadata"]
         if r["original_folder"] in folders],
        key=lambda row: row[0],
    )
    if (len(rows) != expected_count or
            any(classify(source, path) != "cheat" for path, _, _ in rows)):
        raise ValueError("Source-folder gap partition disagrees with metadata audit")
    return rows


SHA = re.compile(r"^[0-9a-f]{40}$")


def classify(source, path):
    if source == "leumas-gamecube-wiird-zips" and re.fullmatch(r"^(?:NTSC-J codes\.zip|NTSC-U Codes\.zip|PAL codes\.zip)$", path):
        return "cheat"
    if source == "katsumaki-wiiu-xml" and re.fullmatch(r"^[^/]+/[^/]+\.xml$", path):
        return "cheat"
    if source == "ucodes-wiiu-json" and re.fullmatch(r"^views/pages/cheats/[0-3]/codes\.json$", path):
        return "cheat"
    if source in SCOUT_GROUPS:
        return "cheat" if re.fullmatch(SCOUT_MATCHERS[source], path) else None
    if source in MIRROR_GAP_GROUPS:
        origin, folders, _, _ = MIRROR_GAP_GROUPS[source]
        segments = path.split("/")
        if (origin == "gcmcom-libretro-legacy" and len(segments) == 3
                and segments[0] == "cht" and segments[1] in folders
                and segments[2].endswith(".cht")):
            return "cheat"
        if (origin == "smoker1-retroarch-174" and len(segments) >= 2
                and segments[0] in folders and path.endswith(".cht")):
            return "cheat"
        return None
    if source == "duckstation-chtdb":
        if re.fullmatch(r"cheats/[^/]+\.cht", path):
            return "cheat"
        if re.fullmatch(r"patches/[^/]+\.cht", path):
            return "patch"
    if source == "crashgg-ps1-chtdb-delta":
        if re.fullmatch(r"cheats/[^/]+\.cht", path):
            return "cheat"
        if re.fullmatch(r"patches/[^/]+\.cht", path):
            return "patch"
    if source == "deadskullz-nds-formats" and re.fullmatch(
            r"Cheat Databases/(?:SCC Cheats\.7z|cheat\.dat|cheat\.db|cheat_EN\.db|cheats\.dat|cheats\.xml|ezarcode\.dat|user\.evoCheats|usrcheat\.dat)", path):
        return "cheat"
    if source == "saramagrean-psp-cwcheat" and path == "cheat.db":
        return "cheat"
    if source == "shadowninja-ps2-pnach-delta" and re.fullmatch(r"[^/]+\.pnach", path):
        return "cheat"
    if source == "libretro-mame2010-cheat-archive" and path == "metadata/cheat.zip":
        return "cheat"
    if source == "libretro-mame2003plus-cheat-archive" and path == "metadata/cheat.dat":
        return "cheat"
    if source == "smoker1-fba-official-ini" and re.fullmatch(r"FB Alpha Official Cheats/[^/]+\.ini", path):
        return "cheat"
    if source == "pacnpal-mupen-n64-cheatfile" and path == "mupencheat.txt":
        return "cheat"
    if source == "xenia-canary-game-patches" and re.fullmatch(r"patches/[^/]+\.patch\.toml", path):
        return "patch"
    if source == "pcsx2-official-patches" and re.fullmatch(r"patches/[^/]+\.pnach", path):
        return "patch"
    if source == "r0ah-vitacheat" and re.fullmatch(r"db/[^/]+\.psv(?:-NSFW)?", path):
        return "cheat"
    if source == "shumnot-vitacheat" and re.fullmatch(r"[^/]+\.psv", path):
        return "cheat"
    if source == "bucanero-dreamcast-cheats" and re.fullmatch(r"[^/]+\.md", path) and path != "README.md":
        return "cheat"
    if source == "ademola-switch-native":
        if (re.fullmatch(r"Titles/[0-9a-fA-F]{16}/cheats/[0-9a-fA-F]{16}\.txt", path)
                or re.fullmatch(r"NX-60FPS-RES-GFX-Cheats/titles/[0-9a-fA-F]{16}/cheats/[0-9a-fA-F]{16}\.txt", path)):
            return "cheat"
        if path.endswith((".pchtxt", ".ips")):
            return "patch"
    if source == "ademola-3ds-native" and re.fullmatch(r"[^/]+/000400[0-9a-fA-F]{10}\.txt", path):
        return "cheat"
    if source.startswith("rogamer97-asm-"):
        prefix = {"3ds": "3DS", "ds": "DS", "gamecube": "GameCube",
                  "switch": "Switch", "wii": "Wii"}.get(source.removeprefix("rogamer97-asm-"))
        if prefix and path.startswith(prefix + "/"):
            if path.endswith(".md") and path.rsplit("/", 1)[-1].lower() != "readme.md":
                return "cheat"
            if path.endswith((".pchtxt", ".ips", ".s")):
                return "patch"
    if source == "dospamu-rpcs3-artemis-patches" and re.fullmatch(
            r"(?:USERLIST|PSXPlace Confirmed|Working Artemis Patches)/[^/]+\.ncl", path):
        return "patch"
    if source == "chidreams-rpcs3-patches" and path and not path.startswith(".") and "/" not in path:
        return "patch"
    if source == "apelfactory-ppsspp-cheatdb" and path == "cheat.db":
        return "cheat"
    if source == "real-f00-usrcheat-ds" and path == "usrcheat.dat":
        return "cheat"
    if source == "hamlet-switch-cheatdb":
        if re.fullmatch(r"(?:cheats|cheats_gbatemp)/[0-9a-fA-F]{16}\.json", path):
            return "cheat"
        if re.fullmatch(r"cheats_gfx/[0-9a-fA-F]{16}\.json", path):
            return "patch"
    if source == "ibnux-switch-cheatdb" and re.fullmatch(r"atmosphere/titles/[0-9a-fA-F]{16}/cheats/[0-9a-fA-F]{16}\.txt", path):
        return "cheat"
    if source == "sthetix-switch-mirror":
        if re.fullmatch(r"(?:cheats|cheats_gbatemp)/[0-9a-fA-F]{16}\.json", path):
            return "cheat"
        if re.fullmatch(r"cheats_gfx/[0-9a-fA-F]{16}\.json", path):
            return "patch"
    if source == "journeyover-3ds-ctrpf" and re.fullmatch(r"Cheats/[^/]+/[0-9a-fA-F]{16}\.txt", path):
        return "cheat"
    if source in ("daowace-pcsx2-consolidated", "xs1l3n7x-pcsx2") and re.fullmatch(r"cheats/[^/]+\.pnach", path):
        return "cheat"
    if source == "ps2-widescreen-staging" and re.fullmatch(r"(?:PNACH|PNACH_WITH_ID)/[^/]+\.pnach", path):
        return "patch"
    return None


def make_index(source, tree):
    spec = SOURCES[source]
    if tree.get("truncated") or not isinstance(tree.get("tree"), list):
        raise ValueError("Git tree incomplete")
    rows = sorted(
        [[item["path"], item["sha"], item["size"]]
         for item in tree["tree"]
         if item.get("type") == "blob" and classify(source, item.get("path", ""))],
        key=lambda row: row[0],
    )
    if source in MIRROR_GAP_GROUPS:
        # Compare the selected original paths against a separately pinned
        # snapshot of three-corpus byte-identity gaps. Never fetch cheat bodies.
        expected = pinned_mirror_gap_rows(source)
        lookup_by_path = {path: (sha, size) for path, sha, size in rows}
        if any(lookup_by_path.get(path) != (sha, size)
               for path, sha, size in expected):
            raise ValueError("Historical mirror tree disagrees with pinned gap file hashes")
        origin = MIRROR_GAP_GROUPS[source][0]
        _repo, _commit, original_total = MIRROR_ORIGINS[origin]
        if sum(item.get("type") == "blob" and item.get("path", "").endswith(".cht")
               for item in tree["tree"]) != original_total:
            raise ValueError("Historical source CHT file count changed")
        rows = expected
    if source in SCOUT_GROUPS:
        if rows != pinned_scout_rows(source):
            raise ValueError("Pinned scout Git tree disagrees with original source inventory: " + source)
    if source == "crashgg-ps1-chtdb-delta":
        # This is an exact-byte delta of another pinned external collection:
        # do not publish thousands of unchanged mirror paths as new leads.
        parent = json.loads((ROOT / FOLDER / "duckstation-chtdb.json").read_text("utf-8"))
        validate("duckstation-chtdb", parent)
        known = {sha for _, sha, _ in parent["files"]}
        if len(rows) != 4485:
            raise ValueError("Pinned derivative PS1 tree no longer has 4,485 native files")
        rows = [r for r in rows if r[1] not in known]
        if len(rows) != 268 or count_types(source, rows) != {"cheat": 263, "patch": 5}:
            raise ValueError("Pinned PS1 delta no longer contains 268 original byte variants")
    if source == "shadowninja-ps2-pnach-delta":
        parent = json.loads((ROOT / FOLDER / "xs1l3n7x-pcsx2.json").read_text("utf-8"))
        validate("xs1l3n7x-pcsx2", parent)
        original = len(rows)
        known = {sha for _, sha, _ in parent["files"]}
        matches = sum(sha in known for _, sha, _ in rows)
        excluded_empty = sum(size == 0 and sha not in known for _, sha, size in rows)
        rows = [r for r in rows if r[1] not in known and r[2] > 0]
        if (original != 628 or matches != 579 or excluded_empty != 1
                or len(rows) != 48 or count_types(source, rows) != {"cheat": 48}):
            raise ValueError("PS2 derivative source no longer matches pinned 579+1 gap audit")
    return {
        "schema_version": 1, "id": source,
        "repository": "https://github.com/" + spec["repository"],
        "snapshot_commit": spec["commit"],
        "git_tree_sha": tree["sha"],
        "platform": spec["platform"],
        "rights_status": "link_only_rights_review",
        "rights_note": spec["rights_note"],
        "filename_scope": "source-native cheat and patch files only; source contents intentionally omitted",
        "file_counts": count_types(source, rows),
        "files": rows,
    }


def count_types(source, rows):
    counts = {}
    for path, _, _ in rows:
        kind = classify(source, path)
        if kind is None:
            raise ValueError("Invalid source-native path: " + path)
        counts[kind] = counts.get(kind, 0) + 1
    return counts


def validate(source, index):
    spec = SOURCES[source]
    if (index.get("schema_version") != 1 or index.get("id") != source
            or index.get("repository") != "https://github.com/" + spec["repository"]
            or index.get("snapshot_commit") != spec["commit"]
            or index.get("platform") != spec["platform"]
            or index.get("rights_status") != "link_only_rights_review"
            or index.get("rights_note") != spec["rights_note"]
            or index.get("filename_scope") != "source-native cheat and patch files only; source contents intentionally omitted"
            or not SHA.fullmatch(index.get("git_tree_sha", ""))):
        raise ValueError(f"Metadata or rights status mismatch for {source}")
    rows = index.get("files")
    if not isinstance(rows, list):
        raise ValueError("Missing source file inventory")
    previous = ""
    for row in rows:
        if not (isinstance(row, list) and len(row) == 3):
            raise ValueError("Unexpected field in a metadata-only file record")
        path, blob, size = row
        if (not isinstance(path, str) or "\x00" in path
                or path.startswith("/") or "\\" in path
                or any(seg in ("", ".", "..") for seg in path.split("/"))
                or path <= previous or classify(source, path) is None
                or not isinstance(blob, str) or not SHA.fullmatch(blob)
                or not isinstance(size, int) or isinstance(size, bool) or size < 0):
            raise ValueError(f"Unsafe or invalid record: {row!r}")
        previous = path
    if source in MIRROR_GAP_GROUPS and rows != pinned_mirror_gap_rows(source):
        raise ValueError("Historical mirror gap paths/hashes changed")
    if count_types(source, rows) != spec["expected"] or index.get("file_counts") != spec["expected"]:
        raise ValueError("Pinned source counts changed for " + source)
    if set(index) != {"schema_version", "id", "repository", "snapshot_commit",
                      "git_tree_sha", "platform", "rights_status", "rights_note",
                      "filename_scope", "file_counts", "files"}:
        raise ValueError("Unexpected manifest fields may indicate code payload leakage")


def source_link(index, path):
    encoded = urllib.parse.quote(path, safe="/")
    return index["repository"] + "/blob/" + index["snapshot_commit"] + "/" + encoded


def lookup(indexes, needle, platform=None, limit=20):
    needle = needle.casefold()
    found = []
    for index in indexes:
        if platform and platform != index["platform"]:
            continue
        for path, blob, size in index["files"]:
            if needle not in path.casefold():
                continue
            found.append({"platform": index["platform"], "source": index["id"],
                          "kind": classify(index["id"], path), "path": path,
                          "git_blob_sha": blob, "bytes": size,
                          "upstream_url": source_link(index, path),
                          "redistribution": "link_only_rights_review"})
            if len(found) >= limit:
                return found
    return found


def load(root):
    result = []
    for source in SOURCES:
        doc = json.loads((root / FOLDER / (source + ".json")).read_text("utf-8"))
        validate(source, doc)
        result.append(doc)
    return result


def refresh(root):
    for source, spec in SOURCES.items():
        url = "https://api.github.com/repos/" + spec["repository"] + "/git/trees/" + spec["commit"] + "?recursive=1"
        request = urllib.request.Request(url, headers={
            "Accept": "application/vnd.github+json",
            "User-Agent": "Cheatarium-external-bulk-metadata-index",
        })
        with urllib.request.urlopen(request, timeout=60) as response:
            upstream = json.load(response)
        index = make_index(source, upstream)
        validate(source, index)
        dest = root / FOLDER / (source + ".json")
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(json.dumps(index, indent=2, ensure_ascii=False) + "\n", "utf-8")
        print(f"Refreshed link-only source {source}: {len(index['files'])} filename/hash records")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--refresh", action="store_true")
    mode.add_argument("--search", metavar="TEXT")
    parser.add_argument("--platform", choices=sorted({x["platform"] for x in SOURCES.values()}))
    parser.add_argument("--limit", type=int, default=20)
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args()
    if args.limit < 1 or args.limit > 100:
        parser.error("--limit must be in [1, 100]")
    if args.refresh:
        refresh(args.root)
    else:
        indexes = load(args.root)
        if args.check:
            for doc in indexes:
                print(f"VALID metadata only: {doc['id']} {len(doc['files'])} original upstream paths")
        else:
            print(json.dumps({"query": args.search, "results": lookup(indexes, args.search, args.platform, args.limit),
                              "note": "External source links, not archived or independently gameplay-verified codes."}, indent=2))


if __name__ == "__main__":
    main()
