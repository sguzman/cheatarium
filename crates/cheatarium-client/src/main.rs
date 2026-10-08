//! Read-only console-cheat lookup for a locally downloaded Cheatarium index.
use cheatarium_client::load_platform;
use std::env;
use std::error::Error;
use std::path::PathBuf;

fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut root = PathBuf::from("generated/v1");
    let mut platform = None;
    let mut title = None;
    let mut json = false;
    let mut limit = 10usize;
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("search") => {}
        _ => {
            eprintln!("Usage: cheatarium-query search --db generated/v1 --platform snes --title Mario [--limit 10] [--json]");
            return Err("Expected search subcommand".into());
        }
    }
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--db" => root = PathBuf::from(args.next().ok_or("--db needs a path")?),
            "--platform" => platform = Some(args.next().ok_or("--platform needs a value")?),
            "--title" => title = Some(args.next().ok_or("--title needs a value")?),
            "--limit" => {
                limit = args.next().ok_or("--limit needs an integer")?.parse()?;
                if !(1..=100).contains(&limit) {
                    return Err("--limit must be 1..100".into());
                }
            }
            "--json" => json = true,
            _ => return Err(format!("Unknown option: {arg}").into()),
        }
    }
    let platform = platform.ok_or("Please provide --platform")?;
    let title = title.ok_or("Please provide --title")?;
    let bundle = load_platform(root, &platform)?;
    let hits = bundle.search_title(&title);
    let total = hits.len();
    if json {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "schema": "cheatarium.query.v1",
            "platform": platform,
            "query": title,
            "candidate_only": true,
            "total_source_records": total,
            "records": hits.into_iter().take(limit).collect::<Vec<_>>()
        }))?);
    } else {
        println!("{total} candidate source records for {title:?} on {platform}");
        println!("WARNING: title matches are suggestions; ROM/build compatibility is unverified.");
        for hit in hits.into_iter().take(limit) {
            let code_count = hit.codes.iter().filter(|x| x.is_code()).count();
            println!("- {} ({code_count} codes; {})", hit.raw_filename, hit.provenance.source_id);
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("cheatarium-query: {e}");
        std::process::exit(1);
    }
}
