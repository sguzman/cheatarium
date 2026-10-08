//! Local-only SNES ZIP inventory generator. No extraction and no network.
use cheatarium_client::inventory::scan_directory;
use std::env;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut rom_dir = None;
    let mut output = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--rom-dir" => {
                rom_dir = Some(PathBuf::from(
                    args.next().ok_or("--rom-dir requires a directory")?,
                ))
            }
            "--out" => {
                output = Some(PathBuf::from(
                    args.next().ok_or("--out requires a JSON filename")?,
                ))
            }
            "--help" | "-h" => {
                println!("cheatarium-inventory --rom-dir /path/to/snes-zips --out /private/snes-inventory.json");
                return Ok(());
            }
            _ => return Err(format!("Unknown inventory option: {arg}").into()),
        }
    }
    let dir = rom_dir.ok_or("Specify --rom-dir")?;
    let output = output.ok_or("Specify --out; this tool never writes an inventory implicitly")?;
    let report = scan_directory(dir)?;
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, &report)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    println!(
        "Scanned {} ZIPs: {} SNES members hashed, {} issues. Metadata only: {}",
        report.archives_scanned,
        report.rom_members_hashed,
        report.issues.len(),
        output.display()
    );
    if !report.issues.is_empty() {
        eprintln!("Warning: some ZIPs or entries could not be read; inspect the issues field.");
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("cheatarium-inventory: {error}");
        std::process::exit(1);
    }
}
