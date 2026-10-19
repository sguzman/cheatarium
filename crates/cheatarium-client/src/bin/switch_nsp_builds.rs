//! One-shot, offline NSP program Build ID inspection using local nstool and hactool.
//! Reads only user-selected packages, stages files in a private temporary directory,
//! and never changes Eden, installed cheats, saves, or the source NSP.
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

type ErrBox = Box<dyn Error + Send + Sync>;
const MAX_NSP_COUNT: usize = 4096;
const MAX_NCA_BYTES: u64 = 12 * 1024 * 1024 * 1024;
const MAX_META_BYTES: u64 = 48 * 1024 * 1024;
const MAX_COMMAND_OUTPUT: usize = 1024 * 1024;

#[derive(Debug)]
struct NcaEntry {
    name: String,
    size: u64,
}
#[derive(Debug, Clone)]
struct Package {
    file: PathBuf,
    title: String,
    version: u32,
    kind: String,
    program: String,
    program_size: u64,
}
#[derive(Serialize)]
struct Found {
    title_id: String,
    source_kind: String,
    version: u32,
    build_id: Option<String>,
    status: String,
}
#[derive(Serialize)]
struct Report {
    format: &'static str,
    read_only: bool,
    source_packages_examined: usize,
    eden_files_written: usize,
    source_files_modified: usize,
    temporary_files_removed_on_exit: bool,
    results: Vec<Found>,
    warnings: Vec<String>,
}

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Result<Self, ErrBox> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        // Avoid RAM-backed /tmp on Arch: an NSP Program NCA may be gigabytes.
        let home = PathBuf::from(env::var_os("HOME").ok_or("HOME_not_set")?);
        let cache = home.join(".cache/cheatarium/nsp-temporary");
        fs::create_dir_all(&cache)?;
        let root = cache.join(format!("cheatarium-nsp-{}-{now}", std::process::id()));
        fs::create_dir(&root)?;
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
        }
        Ok(Self(root))
    }
    fn path(&self, name: &str) -> PathBuf { self.0.join(name) }
}
impl Drop for Scratch {
    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
}
fn safe_output(tool: &str, args: &[String]) -> Result<String, String> {
    let result = Command::new(tool)
        .args(args)
        .output()
        .map_err(|_| format!("{tool}_missing_or_cannot_execute"))?;
    // Do not expose stdout/stderr: external tools may include keys or raw package paths.
    if !result.status.success() {
        return Err(format!("{tool}_exit_failure"));
    }
    if result.stdout.len() > MAX_COMMAND_OUTPUT {
        return Err(format!("{tool}_output_too_large"));
    }
    String::from_utf8(result.stdout).map_err(|_| format!("{tool}_non_utf8_output"))
}
fn hex_identifier(x: &str, n: usize) -> Option<String> {
    let v = x.trim().trim_start_matches("0x");
    (v.len() == n && v.bytes().all(|c| c.is_ascii_hexdigit())).then(|| v.to_ascii_uppercase())
}
fn line_value<'a>(text: &'a str, label: &str) -> Option<&'a str> {
    text.lines().find_map(|line| line.trim().strip_prefix(label).map(str::trim))
}
fn parse_content_metadata(out: &str) -> Option<(String, u32, String, String)> {
    let id = hex_identifier(line_value(out, "TitleId:")?, 16)?;
    let type_name = line_value(out, "Type:")?;
    let kind = type_name.split_whitespace().next()?.to_ascii_lowercase();
    if kind != "application" && kind != "patch" { return None; }
    let ver_text = line_value(out, "Version:")?;
    let raw = ver_text.split("(v").nth(1)?.split(')').next()?;
    let version = raw.parse::<u32>().ok()?;
    let title = if kind == "patch" {
        hex_identifier(line_value(out, "ApplicationId:")?, 16)?
    } else { id.clone() };
    if !title.starts_with("01") { return None; }
    let contents = out.split("ContentInfo:").nth(1)?;
    let mut program = None;
    let mut awaiting_id = false;
    for line in contents.lines() {
        let part = line.trim();
        if part.starts_with("ContentMetaInfo:") || part.starts_with("Digest:") { break; }
        if let Some(ty) = part.strip_prefix("Type:") {
            awaiting_id = ty.trim().starts_with("Program ");
        } else if awaiting_id {
            if let Some(value) = part.strip_prefix("Id:") {
                program = hex_identifier(value, 32).map(|s| format!("{}.nca", s.to_lowercase()));
                break;
            }
        }
    }
    Some((title, version, kind, program?))
}
fn scan_pfs0(path: &Path) -> Result<Vec<NcaEntry>, ErrBox> {
    let mut f = File::open(path)?;
    let len = f.metadata()?.len();
    let mut hdr = [0u8; 16];
    f.read_exact(&mut hdr)?;
    if &hdr[..4] != b"PFS0" { return Err("not_pfs0_nsp".into()); }
    let count = u32::from_le_bytes(hdr[4..8].try_into()?) as usize;
    let str_size = u32::from_le_bytes(hdr[8..12].try_into()?) as usize;
    if count == 0 || count > 16384 || str_size > 4 * 1024 * 1024 {
        return Err("invalid_pfs0_header".into());
    }
    let entries_size = count.checked_mul(24).ok_or("entry_count_overflow")?;
    let payload_offset = 16usize.checked_add(entries_size).and_then(|n| n.checked_add(str_size)).ok_or("nsp_header_overflow")?;
    if payload_offset as u64 > len { return Err("truncated_pfs0_header".into()); }
    let mut table = vec![0u8; entries_size + str_size];
    f.read_exact(&mut table)?;
    let names = &table[entries_size..];
    let mut out = Vec::new();
    for i in 0..count {
        let p = &table[i * 24..(i + 1) * 24];
        let at = u64::from_le_bytes(p[..8].try_into()?);
        let size = u64::from_le_bytes(p[8..16].try_into()?);
        let index = u32::from_le_bytes(p[16..20].try_into()?) as usize;
        if (index >= names.len()) || at.checked_add(size).and_then(|n| n.checked_add(payload_offset as u64)).is_none_or(|end| end > len) {
            return Err("invalid_pfs0_entry_bounds".into());
        }
        let end = names[index..].iter().position(|c| *c == 0).ok_or("invalid_pfs0_name")? + index;
        let name = std::str::from_utf8(&names[index..end])?;
        if name.contains('/') || name.contains('\\') || name.contains("..") { return Err("unsafe_pfs0_entry_name".into()); }
        if name.to_ascii_lowercase().ends_with(".nca") {
            out.push(NcaEntry { name: name.to_owned(), size });
        }
    }
    Ok(out)
}
fn keyfile(specified: Option<PathBuf>) -> Result<PathBuf, ErrBox> {
    let mut candidates = Vec::new();
    if let Some(path) = specified { candidates.push(path); }
    else {
        let home = PathBuf::from(env::var_os("HOME").ok_or("HOME_not_set")?);
        candidates.extend([
            home.join(".switch/prod.keys"),
            home.join(".local/share/eden/keys/prod.keys"),
            home.join(".config/eden/keys/prod.keys"),
        ]);
    }
    for path in candidates {
        if let Ok(meta) = fs::symlink_metadata(&path) {
            if meta.is_file() && !meta.file_type().is_symlink() && meta.len() > 0 && meta.len() < 256 * 1024 { return Ok(path); }
        }
    }
    Err("missing_local_prod_keys_supply_--keyset_path_without_printing_keys".into())
}
fn extract_nca(nsp: &Path, entry: &NcaEntry, output: &Path) -> Result<(), String> {
    if entry.size > MAX_NCA_BYTES { return Err("nca_exceeds_12gib_safety_limit".into()); }
    let args = vec!["-t".into(), "nsp".into(), "-x".into(),
        format!("/{}", entry.name), output.to_string_lossy().to_string(), nsp.to_string_lossy().to_string()];
    safe_output("nstool", &args)?;
    let size = fs::symlink_metadata(output).map_err(|_| "nca_not_extracted".to_owned())?;
    if !size.is_file() || size.is_symlink() || size.len() != entry.size {
        return Err("nca_extract_size_mismatch".into());
    }
    Ok(())
}
fn discover_package(path: &Path, keys: &Path, scratch: &Scratch) -> Result<Option<Package>, String> {
    let items = scan_pfs0(path).map_err(|_| "invalid_nsp".to_owned())?;
    let metas = items.iter().filter(|it| it.name.to_ascii_lowercase().ends_with(".cnmt.nca")).collect::<Vec<_>>();
    let mut found = Vec::new();
    for (i, meta) in metas.iter().enumerate() {
        if meta.size > MAX_META_BYTES { continue; }
        let dest = scratch.path(&format!("meta-{i}.nca"));
        if extract_nca(path, meta, &dest).is_err() { continue; }
        let tree = safe_output("nstool", &[
            "-t".into(), "nca".into(), "-k".into(), keys.to_string_lossy().to_string(),
            "--fstree".into(), dest.to_string_lossy().to_string()
        ]).unwrap_or_default();
        // Metadata NCAs have a small CNMT in one of the archive partitions.
        // A known virtual file tree is required; do not guess content IDs.
        let names = tree.lines().filter_map(|line| {
            line.split_whitespace().find(|token| token.ends_with(".cnmt"))
                .map(|token| token.trim_matches(|c: char| c == '|' || c == '-' || c == '\\').to_owned())
        }).collect::<Vec<_>>();
        for (j, name) in names.iter().enumerate() {
            let cnmt = scratch.path(&format!("metadata-{i}-{j}.cnmt"));
            let virtual_paths = [format!("/0/{name}"), format!("/1/{name}")];
            for vp in virtual_paths {
                let _ = fs::remove_file(&cnmt);
                if safe_output("nstool", &["-t".into(), "nca".into(), "-k".into(), keys.to_string_lossy().to_string(),
                    "-x".into(), vp, cnmt.to_string_lossy().to_string(), dest.to_string_lossy().to_string()]).is_err() { continue; }
                let info = safe_output("nstool", &["-t".into(), "cnmt".into(), cnmt.to_string_lossy().to_string()]).unwrap_or_default();
                if let Some((title, version, kind, program)) = parse_content_metadata(&info) {
                    if let Some(entry) = items.iter().find(|item| item.name.eq_ignore_ascii_case(&program)) {
                        found.push(Package { file: path.to_path_buf(), title, version, kind,
                            program: entry.name.clone(), program_size: entry.size });
                    }
                }
                break;
            }
        }
        let _ = fs::remove_file(dest);
    }
    found.sort_by(|a,b| b.version.cmp(&a.version));
    Ok(found.into_iter().next())
}
fn scan_paths(root: &Path, max: usize) -> Result<Vec<PathBuf>, ErrBox> {
    if !root.is_dir() { return Err("NSP source directory is not readable".into()); }
    let mut files = Vec::new();
    let mut dirs = vec![(root.to_owned(), 0usize)];
    while let Some((dir, depth)) = dirs.pop() {
        if depth > 10 { continue; }
        for item in fs::read_dir(dir)? {
            let item = item?;
            let ty = item.file_type()?;
            if ty.is_symlink() { continue; }
            if ty.is_dir() {
                // DLC packages don't contain a base game's Program NCA.
                if item.file_name().to_string_lossy().eq_ignore_ascii_case("dlc") { continue; }
                dirs.push((item.path(), depth+1));
            } else if ty.is_file() && item.path().extension().is_some_and(|e| e.eq_ignore_ascii_case("nsp")) {
                files.push(item.path());
                if files.len() > max { return Err("too_many_nsp_packages_use_--limit".into()); }
            }
        }
    }
    files.sort();
    Ok(files)
}
fn main_nso_build(path: &Path) -> Result<String, String> {
    let mut f = File::open(path).map_err(|_| "missing_main_nso".to_owned())?;
    if f.metadata().map_err(|_| "missing_main_nso".to_owned())?.len() < 0x60 { return Err("short_main_nso".into()); }
    let mut hdr = [0u8; 0x60];
    f.read_exact(&mut hdr).map_err(|_| "cannot_read_main_nso".to_owned())?;
    if &hdr[..4] != b"NSO0" { return Err("invalid_main_nso_magic".into()); }
    let id = &hdr[0x40..0x50];
    if id.iter().all(|b| *b==0) { return Err("zero_main_build_id".into()); }
    Ok(id.iter().map(|b| format!("{b:02X}")).collect())
}
fn extract_build(package: &Package, base: Option<&Package>, keys: &Path, scratch: &Scratch) -> Result<String, String> {
    let source_item = NcaEntry { name: package.program.clone(), size: package.program_size };
    let update_nca = scratch.path("selected-program.nca");
    extract_nca(&package.file, &source_item, &update_nca)?;
    let base_file = scratch.path("base-program.nca");
    if let Some(original) = base {
        let base_entry = NcaEntry { name: original.program.clone(), size: original.program_size };
        extract_nca(&original.file, &base_entry, &base_file)?;
    }
    let output = scratch.path("exefs");
    fs::create_dir(&output).map_err(|_| "cannot_create_private_exefs".to_owned())?;
    let mut opts = vec!["-i".into(), "-t".into(), "nca".into(), "-k".into(),
        keys.to_string_lossy().to_string(), "--suppresskeys".into(),
        format!("--exefsdir={}", output.display())];
    if base.is_some() { opts.push(format!("--basenca={}", base_file.display())); }
    opts.push(update_nca.to_string_lossy().to_string());
    safe_output("hactool", &opts)?;
    main_nso_build(&output.join("main"))
}
fn run() -> Result<(), ErrBox> {
    let mut roms = PathBuf::from("/drive/roms/switch/roms");
    let mut specified_keys = None;
    let mut limit = 250usize;
    let mut selected = BTreeSet::new();
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--roms" => roms = PathBuf::from(args.next().ok_or("--roms needs a directory")?),
            "--keyset" => specified_keys = Some(PathBuf::from(args.next().ok_or("--keyset needs a path")?)),
            "--limit" => limit = args.next().ok_or("--limit needs a number")?.parse()?,
            "--title-id" => {
                let id = args.next().ok_or("--title-id needs 16-hex ID")?.to_ascii_uppercase();
                if hex_identifier(&id,16).is_none() || !id.starts_with("01") { return Err("invalid_title_id".into()); }
                selected.insert(id);
            }
            "--json" => {},
            "--help" | "-h" => {
                println!("Offline, one-shot NSP Build ID inspector: --roms DIRECTORY --keyset PROD_KEYS --title-id ID (repeat) --limit COUNT --json. Requires nstool and hactool; never writes to Eden or source NSPs.");
                return Ok(());
            }
            _ => return Err("unknown_option".into()),
        }
    }
    if limit == 0 || limit > MAX_NSP_COUNT { return Err("invalid_nsp_scan_limit".into()); }
    let keys = keyfile(specified_keys)?;
    let files = scan_paths(&roms, limit)?;
    let scratch = Scratch::new()?;
    let mut packages = Vec::new();
    let mut warnings = Vec::new();
    for (i, file) in files.iter().enumerate() {
        let temp = Scratch::new()?;
        match discover_package(file, &keys, &temp) {
            Ok(Some(pkg)) if selected.is_empty() || selected.contains(&pkg.title) => packages.push(pkg),
            Ok(_) => {},
            Err(kind) => warnings.push(format!("package_{i}: {kind}")),
        }
    }
    let mut grouped = BTreeMap::<String, Vec<Package>>::new();
    for package in packages { grouped.entry(package.title.clone()).or_default().push(package); }
    let mut results = Vec::new();
    for (title, group) in grouped {
        let mut bases = group.iter().filter(|p| p.kind=="application").collect::<Vec<_>>();
        let mut patches = group.iter().filter(|p| p.kind=="patch").collect::<Vec<_>>();
        bases.sort_by(|a,b| b.version.cmp(&a.version));
        patches.sort_by(|a,b| b.version.cmp(&a.version));
        let selected_pkg = patches.first().copied().or_else(|| bases.first().copied()).ok_or("no_program_source")?;
        let base = if selected_pkg.kind=="patch" { bases.first().copied() } else { None };
        if patches.len()>1 && patches[0].version == patches[1].version && patches[0].program != patches[1].program {
            results.push(Found {title_id:title,source_kind:"patch".into(),version:selected_pkg.version,
                build_id:None,status:"ambiguous_same_version_updates".into()});continue;
        }
        if selected_pkg.kind=="patch" && base.is_none() {
            results.push(Found {title_id:title,source_kind:"patch".into(),version:selected_pkg.version,
                build_id:None,status:"missing_base_program_nca_for_update".into()});continue;
        }
        // Read-only against source and Eden: all extraction is into temporary files.
        let res = extract_build(selected_pkg,base,&keys,&scratch);
        results.push(Found {title_id:title,source_kind:selected_pkg.kind.clone(),
            version:selected_pkg.version,build_id:res.as_ref().ok().cloned(),
            status:res.err().unwrap_or_else(|| "extracted_main_nso_build_id".into())});
        for name in ["selected-program.nca", "base-program.nca"] {
            let _ = fs::remove_file(scratch.path(name));
        }
        let _ = fs::remove_dir_all(scratch.path("exefs"));
    }
    for title in selected {
        if !results.iter().any(|r| r.title_id==title) {
            results.push(Found {title_id:title,source_kind:"none".into(),version:0,build_id:None,status:"not_found_in_scanned_nsp_content_metadata".into()});
        }
    }
    results.sort_by(|a,b| a.title_id.cmp(&b.title_id));
    let report = Report {
        format:"cheatarium-static-nsp-build-inspection-v1",read_only:true,
        source_packages_examined:files.len(),eden_files_written:0,source_files_modified:0,
        temporary_files_removed_on_exit:true,results,warnings
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
fn main() {
    if let Err(e) = run() { eprintln!("cheatarium-switch-nsp-builds: {e}"); std::process::exit(1); }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identifies_program_from_nstool_cnmt() {
        let doc = "[ContentMeta]\n  TitleId:               0x01002da013484800\n  Version:               1.0.1 (v65536)\n  Type:                  Patch (129)\n  PatchMetaExtendedHeader:\n    ApplicationId:         0x01002da013484000\n  ContentInfo:\n    0\n      Type:         Program (1)\n      Id:           0123456789abcdef0123456789abcdef\n      Size:         0x123456\n  Digest: abc";
        assert_eq!(parse_content_metadata(doc),Some(("01002DA013484000".into(),65536,"patch".into(),"0123456789abcdef0123456789abcdef.nca".into())));
    }
    #[test]
    fn rejects_metadata_without_program() {
        let doc="[ContentMeta]\nTitleId: 0x01002da013484000\nVersion: 1.0.0 (v0)\nType: Application (128)\nContentInfo:\n 0\n Type: Control (3)\n Id: 0123456789abcdef0123456789abcdef\n";
        assert_eq!(parse_content_metadata(doc),None);
    }
    #[test]
    fn parses_minimal_pfs0_and_rejects_oob_bounds() {
        let scratch=Scratch::new().unwrap();
        let path=scratch.path("fixture.nsp");
        let name=b"0123456789abcdef0123456789abcdef.cnmt.nca\0";
        let mut raw=Vec::new();raw.extend(b"PFS0");raw.extend(1u32.to_le_bytes());raw.extend((name.len() as u32).to_le_bytes());raw.extend([0u8;4]);
        raw.extend(0u64.to_le_bytes());raw.extend(4u64.to_le_bytes());raw.extend(0u32.to_le_bytes());raw.extend(0u32.to_le_bytes());
        raw.extend(name);raw.extend([1u8;4]);fs::write(&path,&raw).unwrap();
        let found=scan_pfs0(&path).unwrap();assert_eq!(found.len(),1);assert_eq!(found[0].size,4);
        raw.pop();fs::write(&path,&raw).unwrap();assert!(scan_pfs0(&path).is_err());
    }
    #[test]
    fn reads_nso_build_id_at_expected_offset() {
        let scratch=Scratch::new().unwrap();let path=scratch.path("main");
        let mut data=[0u8;0x60];data[..4].copy_from_slice(b"NSO0");
        data[0x40..0x50].copy_from_slice(&[0x54,0x41,0xAF,0x3C,0x89,0xF9,0x7E,0x1D,0,1,2,3,4,5,6,7]);
        fs::write(&path,data).unwrap();
        assert_eq!(main_nso_build(&path).unwrap(),"5441AF3C89F97E1D0001020304050607");
    }
}
