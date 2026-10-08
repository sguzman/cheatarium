//! Local, read-only SHA-256 evidence lookup for a specifically selected file.
//! No ROM content is uploaded or included in Cheatarium releases.
use cheatarium_client::identity::{
    fingerprint_file, load_registry, sha256_valid, Fingerprint, HASH_SCOPE,
};
use std::env;
use std::error::Error;
use std::path::PathBuf;

fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut args = env::args().skip(1);
    let mut platform = None;
    let mut source_file: Option<PathBuf> = None;
    let mut hash = None;
    let mut registry_dir = PathBuf::from("generated/v1/identities");

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--platform" => platform = Some(args.next().ok_or("--platform requires an ID")?),
            "--file" => {
                source_file = Some(PathBuf::from(args.next().ok_or("--file requires a path")?))
            }
            "--sha256" => hash = Some(args.next().ok_or("--sha256 requires a digest")?),
            "--registry" => {
                registry_dir = PathBuf::from(args.next().ok_or("--registry requires a directory")?)
            }
            "--help" | "-h" => {
                println!("cheatarium-fingerprint --platform snes (--file /local/game.sfc | --sha256 HASH) [--registry generated/v1/identities]");
                return Ok(());
            }
            _ => return Err(format!("Unknown fingerprint option {arg}").into()),
        }
    }
    if source_file.is_some() == hash.is_some() {
        return Err("Specify exactly one of --file or --sha256".into());
    }
    let platform = platform.ok_or("Missing --platform")?;
    let fingerprint = if let Some(file) = source_file {
        fingerprint_file(file)?
    } else {
        let sha256 = hash.ok_or("Missing SHA-256 digest")?;
        if !sha256_valid(&sha256) {
            return Err("Expected lowercase 64-character SHA-256 digest".into());
        }
        Fingerprint {
            sha256,
            byte_length: 0,
            hash_scope: HASH_SCOPE.to_owned(),
        }
    };
    let registry = load_registry(registry_dir, &platform)?;
    let matching = registry.lookup_with_length(
        &fingerprint.sha256,
        (fingerprint.byte_length > 0).then_some(fingerprint.byte_length),
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema": "cheatarium.fingerprint.v1",
            "platform": platform,
            "fingerprint": fingerprint,
            "size_known": fingerprint.byte_length != 0,
            "evidence_lookup": matching,
            "filename_matching_used": false,
            "cheats_activated": false,
        }))?
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("cheatarium-fingerprint: {error}");
        std::process::exit(1);
    }
}
