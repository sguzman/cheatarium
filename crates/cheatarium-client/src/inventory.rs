//! Local read-only inventory of SNES ROM members inside ZIP archives.
//! Output is metadata, not ROM bytes or a verification of release identity.
use crate::identity::{fingerprint_reader, HASH_SCOPE};
use crate::Result;
use serde::Serialize;
use std::fs::{self, File};
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use zip::ZipArchive;

const MAX_ROM_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ZIP_MEMBERS: usize = 100_000;
const MAX_ARCHIVES: usize = 100_000;
const MAX_RECORDS: usize = 500_000;

#[derive(Debug, Serialize)]
pub struct RomMember {
    /// Paths are relative to the selected directory; no absolute home paths.
    pub archive: String,
    /// Original relative ZIP member name; never extracted to disk.
    pub member: String,
    pub sha256: String,
    pub byte_length: u64,
    /// Weak heuristic only; never removes a header or alters the hash.
    pub possible_512_byte_copier_header_by_length_only: bool,
}

#[derive(Debug, Serialize)]
pub struct InventoryIssue {
    pub archive: String,
    pub member: Option<String>,
    /// A fixed reason code; never includes file contents or absolute paths.
    pub reason: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ZipInventory {
    pub schema_version: u32,
    pub platform: &'static str,
    pub hash_scope: &'static str,
    pub identity_status: &'static str,
    pub archives_scanned: usize,
    pub rom_members_hashed: usize,
    pub entries: Vec<RomMember>,
    pub issues: Vec<InventoryIssue>,
}

impl Default for ZipInventory {
    fn default() -> Self {
        Self {
            schema_version: 1,
            platform: "snes",
            hash_scope: HASH_SCOPE,
            identity_status: "fingerprints_only_no_release_or_cheat_compatibility_claims",
            archives_scanned: 0,
            rom_members_hashed: 0,
            entries: Vec::new(),
            issues: Vec::new(),
        }
    }
}

fn zip_extension(path: &Path) -> bool {
    path.extension()
        .is_some_and(|s| s.eq_ignore_ascii_case("zip"))
}

fn is_snes_rom(name: &str) -> bool {
    let ext = name.rsplit('.').next().unwrap_or("");
    ext.eq_ignore_ascii_case("sfc") || ext.eq_ignore_ascii_case("smc")
}

fn safe_member(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('/')
        && !name.contains('\\')
        && !name.contains('\0')
        && name.split('/').all(|part| !matches!(part, "" | "." | ".."))
}

fn issue(inv: &mut ZipInventory, archive: &str, member: Option<String>, reason: &'static str) {
    inv.issues.push(InventoryIssue {
        archive: archive.to_owned(),
        member,
        reason,
    });
}

/// ZIP processing is completely streaming: each ROM is decoded into a small
/// fixed-size read buffer, hashed, and dropped without any on-disk extraction.
/// An archive with unsupported encryption/compression is reported as an issue.
pub fn scan_zip<R: Read + Seek>(reader: R, archive: &str, inv: &mut ZipInventory) {
    inv.archives_scanned += 1;
    let mut zip = match ZipArchive::new(reader) {
        Ok(zip) => zip,
        Err(_) => {
            issue(inv, archive, None, "invalid_zip_archive");
            return;
        }
    };
    if zip.len() > MAX_ZIP_MEMBERS {
        issue(inv, archive, None, "zip_member_count_limit");
        return;
    }
    for index in 0..zip.len() {
        let mut member = match zip.by_index(index) {
            Ok(member) => member,
            Err(_) => {
                issue(inv, archive, None, "unreadable_zip_member");
                continue;
            }
        };
        if member.is_dir() || !is_snes_rom(member.name()) {
            continue;
        }
        let name = member.name().to_owned();
        if !safe_member(&name) {
            issue(inv, archive, None, "unsafe_zip_member_name");
            continue;
        }
        if inv.entries.len() >= MAX_RECORDS {
            issue(inv, archive, Some(name), "inventory_entry_limit");
            return;
        }
        let declared_size = member.size();
        if declared_size > MAX_ROM_BYTES {
            issue(inv, archive, Some(name), "rom_size_limit");
            continue;
        }
        let fingerprint = match fingerprint_reader((&mut member).take(MAX_ROM_BYTES + 1)) {
            Ok(fingerprint) => fingerprint,
            Err(_) => {
                issue(inv, archive, Some(name), "rom_stream_or_crc_error");
                continue;
            }
        };
        if fingerprint.byte_length != declared_size || fingerprint.byte_length > MAX_ROM_BYTES {
            issue(inv, archive, Some(name), "rom_size_mismatch_or_limit");
            continue;
        }
        let possible_header = declared_size >= 1536 && declared_size % 1024 == 512;
        inv.entries.push(RomMember {
            archive: archive.to_owned(),
            member: name,
            sha256: fingerprint.sha256,
            byte_length: fingerprint.byte_length,
            possible_512_byte_copier_header_by_length_only: possible_header,
        });
    }
}

/// Recurse through the user-selected directory without following nested
/// symlinks. Non-ZIP files are never opened, and ZIP contents are never saved.
pub fn scan_directory(root: impl AsRef<Path>) -> Result<ZipInventory> {
    let root = root.as_ref();
    if !root.is_dir() {
        return Err("Expected an existing directory containing ZIP archives".into());
    }
    let mut result = ZipInventory::default();
    let mut pending: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let mut entries = fs::read_dir(&dir)?.collect::<std::io::Result<Vec<_>>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries.into_iter().rev() {
            let path = entry.path();
            let kind = fs::symlink_metadata(&path)?.file_type();
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                pending.push(path);
                continue;
            }
            if !kind.is_file() || !zip_extension(&path) {
                continue;
            }
            if result.archives_scanned >= MAX_ARCHIVES {
                return Err("Too many ZIP files in selected directory".into());
            }
            let relative = path
                .strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/");
            match File::open(&path) {
                Ok(file) => scan_zip(file, &relative, &mut result),
                Err(_) => issue(&mut result, &relative, None, "unreadable_zip_archive"),
            }
        }
    }
    if result.archives_scanned == 0 {
        return Err("No ZIP archives found inside selected directory".into());
    }
    result
        .entries
        .sort_by(|a, b| (&a.archive, &a.member).cmp(&(&b.archive, &b.member)));
    result.rom_members_hashed = result.entries.len();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    fn fixture(entries: &[(&str, &[u8])]) -> Cursor<Vec<u8>> {
        let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, bytes) in entries {
            archive
                .start_file(
                    *name,
                    SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
                )
                .unwrap();
            archive.write_all(bytes).unwrap();
        }
        archive.finish().unwrap()
    }

    #[test]
    fn hashes_decompressed_snes_bytes_without_extraction() {
        let mut inv = ZipInventory::default();
        let bytes = fixture(&[
            ("Super Mario World (USA).sfc", b"abc"),
            ("ignore.txt", b"unrelated"),
            ("nested/Another Game.SMC", b"xyz"),
        ]);
        scan_zip(bytes, "games/a.zip", &mut inv);
        assert!(inv.issues.is_empty());
        assert_eq!(inv.entries.len(), 2);
        assert_eq!(inv.entries[0].archive, "games/a.zip");
        assert_eq!(
            inv.entries[0].sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(inv.entries[0].byte_length, 3);
        assert_eq!(inv.entries[1].member, "nested/Another Game.SMC");
        assert_eq!(inv.hash_scope, HASH_SCOPE);
    }

    #[test]
    fn copier_header_size_is_only_a_hint_and_does_not_change_hash() {
        let rom = vec![42_u8; 1536];
        let mut inv = ZipInventory::default();
        scan_zip(fixture(&[("fixture.smc", &rom)]), "fixture.zip", &mut inv);
        assert_eq!(inv.entries.len(), 1);
        assert!(inv.entries[0].possible_512_byte_copier_header_by_length_only);
        assert_eq!(
            inv.entries[0].sha256,
            fingerprint_reader(rom.as_slice()).unwrap().sha256
        );
    }

    #[test]
    fn invalid_zips_and_unsafe_member_names_are_never_extracted() {
        let mut inv = ZipInventory::default();
        scan_zip(Cursor::new(b"not a ZIP".to_vec()), "bad.zip", &mut inv);
        assert_eq!(inv.issues[0].reason, "invalid_zip_archive");
        assert!(!safe_member("../game.sfc"));
        assert!(!safe_member("/game.sfc"));
        assert!(!safe_member("folder\\game.sfc"));
        assert!(safe_member("folder/game.sfc"));
        assert!(!is_snes_rom("readme.txt"));
    }
}
