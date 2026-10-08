//! Offline SNES cheat decoding demonstration; never touches a running emulator.
use cheatarium_codecs::{decode_snes, decode_snes_unlabelled};
use std::env;

fn main() {
    let mut args = env::args().skip(1);
    let (Some(command), Some(console), Some(format), Some(code), None) = (
        args.next(),
        args.next(),
        args.next(),
        args.next(),
        args.next(),
    ) else {
        eprintln!(
            "Usage: cheatarium-decode decode snes <game-genie|action-replay|syntax> '<code>'"
        );
        std::process::exit(2);
    };
    if command != "decode" || console != "snes" {
        eprintln!("Only the decode snes command is supported");
        std::process::exit(2);
    }
    let result = if format == "syntax" {
        decode_snes_unlabelled(&code)
    } else {
        decode_snes(&format, &code)
    };
    match result {
        Ok(decoded) => {
            let doc = serde_json::json!({
                "schema_version": 1,
                "console": "snes",
                "source_code": code,
                "executable": false,
                "rom_compatible": false,
                "decoded": decoded,
            });
            match serde_json::to_string_pretty(&doc) {
                Ok(output) => println!("{output}"),
                Err(err) => {
                    eprintln!("Could not format decoded result: {err}");
                    std::process::exit(1);
                }
            }
        }
        Err(err) => {
            eprintln!("Could not strictly decode SNES code: {err:?}");
            std::process::exit(2);
        }
    }
}
