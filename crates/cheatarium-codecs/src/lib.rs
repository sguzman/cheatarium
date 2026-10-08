//! Pure, opt-in SNES device-code decoding.
//!
//! A decoded address and value are NOT permission to write emulator memory.
//! Cartridge revision, patch timing and bus semantics are consumer-specific.
//! No ROMs or emulators are accessed.
use serde::{Deserialize, Serialize};

const GENIE_DIGITS: &[u8; 16] = b"DF4709156BC8A23E";
const MAX_COMPOUND_PARTS: usize = 64;
const MAX_SOURCE_LENGTH: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnesWrite {
    /// 24-bit SNES CPU bus address; six uppercase hex digits.
    pub address_hex: String,
    /// Replacement byte; two uppercase hex digits.
    pub value_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnesDecoded {
    pub format: String,
    pub address_space: String,
    pub compatibility: String,
    /// "declared-file-format" or "code-syntax", never verified compatibility.
    pub interpretation_basis: String,
    /// All parts must be successfully decoded; never apply only part of a group.
    pub writes: Vec<SnesWrite>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    UnsupportedFormat,
    InvalidCode,
    TooManyParts,
    EmptyPart,
}

fn read_game_genie(input: &str) -> Result<SnesWrite, DecodeError> {
    let bytes = input.as_bytes();
    if bytes.len() != 9 || bytes[4] != b'-' {
        return Err(DecodeError::InvalidCode);
    }
    let mut parsed = 0_u32;
    for (i, &c) in bytes.iter().enumerate() {
        if i == 4 {
            continue;
        }
        let upper = c.to_ascii_uppercase();
        let digit = GENIE_DIGITS
            .iter()
            .position(|&v| v == upper)
            .ok_or(DecodeError::InvalidCode)? as u32;
        parsed = (parsed << 4) | digit;
    }
    let value = (parsed >> 24) as u8;
    let a = parsed & 0x00ff_ffff;
    // SNES Game Genie address permutation also used by Snes9x and bsnes.
    let address = ((a & 0x003c00) << 10)
        | ((a & 0x00003c) << 14)
        | ((a & 0xf00000) >> 8)
        | ((a & 0x000003) << 10)
        | ((a & 0x00c000) >> 6)
        | ((a & 0x0f0000) >> 12)
        | ((a & 0x0003c0) >> 6);
    Ok(SnesWrite {
        address_hex: format!("{address:06X}"),
        value_hex: format!("{value:02X}"),
    })
}

fn read_action_replay(input: &str) -> Result<SnesWrite, DecodeError> {
    if input.len() != 8 || !input.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(DecodeError::InvalidCode);
    }
    let bits = u32::from_str_radix(input, 16).map_err(|_| DecodeError::InvalidCode)?;
    Ok(SnesWrite {
        address_hex: format!("{:06X}", bits >> 8),
        value_hex: format!("{:02X}", bits & 0xff),
    })
}

/// Decode the complete compound string without touching any emulator.
/// Malformed/wildcard parts reject the entire group; no partial writes escape.
pub fn decode_snes(format: &str, raw: &str) -> Result<SnesDecoded, DecodeError> {
    if !matches!(format, "game-genie" | "action-replay") {
        return Err(DecodeError::UnsupportedFormat);
    }
    if raw.len() > MAX_SOURCE_LENGTH {
        return Err(DecodeError::TooManyParts);
    }
    let parts: Vec<_> = raw.split('+').collect();
    if parts.len() > MAX_COMPOUND_PARTS {
        return Err(DecodeError::TooManyParts);
    }
    let mut writes = Vec::with_capacity(parts.len());
    for part in parts {
        let part = part.trim();
        if part.is_empty() {
            return Err(DecodeError::EmptyPart);
        }
        writes.push(if format == "game-genie" {
            read_game_genie(part)?
        } else {
            read_action_replay(part)?
        });
    }
    Ok(SnesDecoded {
        format: format.to_owned(),
        address_space: "snes-cpu-bus-24-bit".to_owned(),
        compatibility: "unverified-cartridge-build".to_owned(),
        interpretation_basis: "declared-file-format".to_owned(),
        writes,
    })
}

/// Interpret *unlabelled* SNES code text conservatively.
/// Hyphenated Game Genie has a distinctive alphabet; exactly eight hex
/// digits can only be called a raw 24-bit-address/8-bit-value candidate.
/// Never attribute anonymous hex to Pro Action Replay or invent a ROM match.
/// Mixed groups, placeholders and other encodings remain uninterpreted.
pub fn decode_snes_unlabelled(raw: &str) -> Result<SnesDecoded, DecodeError> {
    if raw.len() > MAX_SOURCE_LENGTH {
        return Err(DecodeError::TooManyParts);
    }
    let parts: Vec<_> = raw.split('+').map(str::trim).collect();
    if parts.len() > MAX_COMPOUND_PARTS {
        return Err(DecodeError::TooManyParts);
    }
    if parts.iter().any(|part| part.is_empty()) {
        return Err(DecodeError::EmptyPart);
    }
    let all_game_genie = parts.iter().all(|part| {
        part.len() == 9
            && part.as_bytes().get(4) == Some(&b'-')
            && part.bytes().enumerate().all(|(i, ch)| {
                i == 4 || GENIE_DIGITS.contains(&ch.to_ascii_uppercase())
            })
    });
    let all_raw_hex = parts.iter().all(|part| {
        part.len() == 8 && part.bytes().all(|ch| ch.is_ascii_hexdigit())
    });
    let mut decoded = if all_game_genie {
        decode_snes("game-genie", raw)?
    } else if all_raw_hex {
        decode_snes("action-replay", raw)?
    } else {
        return Err(DecodeError::InvalidCode);
    };
    if all_raw_hex {
        decoded.format = "raw-snes-address-value".to_owned();
    }
    decoded.interpretation_basis = "code-syntax".to_owned();
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_super_mario_world_vectors() {
        let one = decode_snes("game-genie", "DDB4-6F07").unwrap();
        let nine = decode_snes("game-genie", "d6b4-6f07").unwrap();
        assert_eq!(one.writes[0].address_hex, "009E25");
        assert_eq!(one.writes[0].value_hex, "00");
        assert_eq!(nine.writes[0].address_hex, "009E25");
        assert_eq!(nine.writes[0].value_hex, "08");
        let infinite = decode_snes("game-genie", "C222-D4DD").unwrap();
        assert_eq!(
            infinite.writes[0],
            SnesWrite {
                address_hex: "00D0D8".to_owned(),
                value_hex: "AD".to_owned(),
            }
        );
    }

    #[test]
    fn action_replay_and_compound() {
        let data = decode_snes("action-replay", "7E1E6B14 + 7F80CAFF").unwrap();
        assert_eq!(data.writes.len(), 2);
        assert_eq!(data.writes[0].address_hex, "7E1E6B");
        assert_eq!(data.writes[0].value_hex, "14");
        assert_eq!(data.writes[1].address_hex, "7F80CA");
        assert_eq!(data.writes[1].value_hex, "FF");
    }

    #[test]
    fn wildcard_and_partial_groups_rejected() {
        assert_eq!(
            decode_snes("action-replay", "7FC136XX"),
            Err(DecodeError::InvalidCode)
        );
        assert_eq!(
            decode_snes("action-replay", "7E1E6B14+7FC136XX"),
            Err(DecodeError::InvalidCode)
        );
        assert_eq!(
            decode_snes("game-genie", "DDB4-6F07+"),
            Err(DecodeError::EmptyPart)
        );
        assert_eq!(
            decode_snes("game-genie", "DDB4-6F0X"),
            Err(DecodeError::InvalidCode)
        );
        assert_eq!(
            decode_snes("gameshark", "7E1E6B14"),
            Err(DecodeError::UnsupportedFormat)
        );
    }

    #[test]
    fn unlabeled_game_genie_is_syntax_only() {
        let result = decode_snes_unlabelled("ddB4-6f07").unwrap();
        assert_eq!(result.format, "game-genie");
        assert_eq!(result.interpretation_basis, "code-syntax");
        assert_eq!(result.writes[0].address_hex, "009E25");
        assert_eq!(result.writes[0].value_hex, "00");
    }

    #[test]
    fn anonymous_hex_is_not_mislabeled_as_action_replay() {
        let result = decode_snes_unlabelled("7E1E6B14+7F80CAFF").unwrap();
        assert_eq!(result.format, "raw-snes-address-value");
        assert_eq!(result.interpretation_basis, "code-syntax");
        assert_eq!(result.writes.len(), 2);
        assert_eq!(result.writes[1].address_hex, "7F80CA");
    }

    #[test]
    fn mixed_or_incomplete_unlabeled_groups_not_interpreted() {
        assert_eq!(decode_snes_unlabelled("DDB4-6F07+7E1E6B14"), Err(DecodeError::InvalidCode));
        assert_eq!(decode_snes_unlabelled("7FC136XX"), Err(DecodeError::InvalidCode));
        assert_eq!(decode_snes_unlabelled("DDB4-6F07+"), Err(DecodeError::EmptyPart));
        assert_eq!(decode_snes_unlabelled("ABCD/1234"), Err(DecodeError::InvalidCode));
        let known = decode_snes("action-replay", "7E1E6B14").unwrap();
        assert_eq!(known.interpretation_basis, "declared-file-format");
    }

    #[test]
    fn bounded_and_zero_vector() {
        let too_long = "DDDD-DDDD+".repeat(65);
        assert_eq!(
            decode_snes("game-genie", &too_long),
            Err(DecodeError::TooManyParts)
        );
        assert_eq!(
            decode_snes("game-genie", "DDDD-DDDD").unwrap().writes[0].value_hex,
            "00"
        );
        assert_eq!(
            decode_snes("action-replay", "00123456").unwrap().writes[0].address_hex,
            "001234"
        );
    }
}
