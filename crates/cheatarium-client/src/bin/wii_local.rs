//! Offline read-only CLI for a user-owned, pinned Wii Gecko sidecar.
//! Source cheat bodies are not present in public Cheatarium artifacts.
use cheatarium_client::local_wii::{default_path, load, query, Filters};
use std::env;
use std::error::Error;
use std::path::PathBuf;

fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut path = default_path();
    let mut game_id = None;
    let mut revision = None;
    let mut variant = None;
    let mut game_title = None;
    let mut group_title = None;
    let mut group_index = None;
    let mut show_code = false;
    let mut json = false;
    let mut limit = 20usize;
    let mut offset = 0usize;
    let mut args = env::args().skip(1);
    while let Some(opt) = args.next() {
        match opt.as_str() {
            "--index" => path = PathBuf::from(args.next().ok_or("--index needs a path")?),
            "--game-id" => game_id = Some(args.next().ok_or("--game-id needs a disc ID")?),
            "--revision" => {
                revision = Some(args.next().ok_or("--revision needs a source revision")?)
            }
            "--variant" => variant = Some(args.next().ok_or("--variant needs a value")?),
            "--game-title" => game_title = Some(args.next().ok_or("--game-title needs text")?),
            "--title" => group_title = Some(args.next().ok_or("--title needs text")?),
            "--group-index" => {
                group_index = Some(args.next().ok_or("--group-index needs a number")?.parse()?)
            }
            "--limit" => limit = args.next().ok_or("--limit needs an integer")?.parse()?,
            "--offset" => offset = args.next().ok_or("--offset needs an integer")?.parse()?,
            "--show-code" => show_code = true,
            "--json" => json = true,
            "--help" | "-h" => {
                println!("Usage: cheatarium-wii-local [--index PATH] [--game-title TEXT] [--game-id RUUE01] [--revision 0] [--variant base|mp1|mp2|mp3] [--title GROUP_TEXT] [--limit 20] [--offset 0] [--json] [--show-code --group-index 1]");
                println!("Reads only an existing private Wii Gecko sidecar. No game file access, code execution, or automatic enabling.");
                return Ok(());
            }
            _ => return Err(format!("Unknown Wii local option: {opt}").into()),
        }
    }
    let local = load(&path)?;
    let filters = Filters {
        game_id: game_id.as_deref(),
        revision: revision.as_deref(),
        variant: variant.as_deref(),
        game_title: game_title.as_deref(),
        group_title: group_title.as_deref(),
        group_index,
        show_code,
        offset,
        limit,
    };
    let result = query(&local, &filters)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!(
            "{} Wii Gecko native groups in {} source files; all unverified and disabled",
            result["matched_native_groups"], result["matched_source_files"]
        );
        for row in result["results"]
            .as_array()
            .ok_or("Invalid private search result")?
        {
            println!(
                "- {} {} {} group {}: {} ({} source code lines)",
                row["claimed_disc_id"].as_str().unwrap_or("?"),
                row["source_revision_token"]
                    .as_str()
                    .unwrap_or("revision-unspecified"),
                row["source_variant_token"].as_str().unwrap_or("base"),
                row["group_index"],
                row["group_title"].as_str().unwrap_or("?"),
                row["native_code_pair_count"],
            );
            println!(
                "  {} — {}",
                row["source_compatibility_warning"]
                    .as_str()
                    .unwrap_or("Source version not verified"),
                row["original_upstream_url"].as_str().unwrap_or("No origin"),
            );
            if show_code {
                for code in row["native_code_lines"]
                    .as_array()
                    .ok_or("Missing original code lines")?
                {
                    println!("{}", code.as_str().ok_or("Invalid native code line")?);
                }
            }
        }
        if !show_code {
            println!("Native code text hidden; use exact --game-id [--revision/--variant] --show-code --group-index N.");
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("cheatarium-wii-local: {error}");
        std::process::exit(1);
    }
}
