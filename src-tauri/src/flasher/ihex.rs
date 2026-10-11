//! Intel HEX parsing into a flat flash image.

/// Parses an Intel HEX file into a memory image starting at address 0.
/// Bytes not covered by any record are left erased (0xFF).
pub fn parse(hex: &str) -> Result<Vec<u8>, String> {
    let mut image: Vec<u8> = Vec::new();
    let mut base: u32 = 0;

    for (index, line) in hex.lines().enumerate() {
        let line_number = index + 1;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let record = line
            .strip_prefix(':')
            .ok_or_else(|| format!("line {line_number}: missing ':' record mark"))?;
        let bytes = decode_hex(record).map_err(|e| format!("line {line_number}: {e}"))?;
        if bytes.len() < 5 {
            return Err(format!("line {line_number}: record too short"));
        }
        let length = bytes[0] as usize;
        if bytes.len() != length + 5 {
            return Err(format!("line {line_number}: record length mismatch"));
        }
        if bytes.iter().fold(0u8, |sum, b| sum.wrapping_add(*b)) != 0 {
            return Err(format!("line {line_number}: bad checksum"));
        }

        let offset = u16::from_be_bytes([bytes[1], bytes[2]]) as u32;
        let data = &bytes[4..4 + length];
        match bytes[3] {
            0x00 => {
                let start = (base + offset) as usize;
                let end = start + data.len();
                if image.len() < end {
                    image.resize(end, 0xFF);
                }
                image[start..end].copy_from_slice(data);
            }
            0x01 => return Ok(image),
            0x02 => base = (u16_from(data, line_number)? as u32) << 4,
            0x04 => base = (u16_from(data, line_number)? as u32) << 16,
            // Start addresses are irrelevant to a bootloader.
            0x03 | 0x05 => {}
            other => {
                return Err(format!(
                    "line {line_number}: unknown record type {other:#04x}"
                ))
            }
        }
    }

    Err("missing end-of-file record".into())
}

fn u16_from(data: &[u8], line_number: usize) -> Result<u16, String> {
    match data {
        [hi, lo] => Ok(u16::from_be_bytes([*hi, *lo])),
        _ => Err(format!(
            "line {line_number}: address record must hold 2 bytes"
        )),
    }
}

fn decode_hex(text: &str) -> Result<Vec<u8>, String> {
    if !text.len().is_multiple_of(2) {
        return Err("odd number of hex digits".into());
    }
    (0..text.len())
        .step_by(2)
        .map(|i| {
            text.get(i..i + 2)
                .and_then(|pair| u8::from_str_radix(pair, 16).ok())
                .ok_or_else(|| format!("invalid hex digits at column {}", i + 2))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_data_records_and_fills_gaps() {
        let hex = ":0400000001020304F2\n:02000800AABB91\n:00000001FF\n";
        assert_eq!(
            parse(hex).unwrap(),
            [0x01, 0x02, 0x03, 0x04, 0xFF, 0xFF, 0xFF, 0xFF, 0xAA, 0xBB]
        );
    }

    #[test]
    fn applies_extended_linear_address() {
        let hex = ":020000040001F9\n:01000000AA55\n:00000001FF\n";
        let image = parse(hex).unwrap();
        assert_eq!(image.len(), 0x10001);
        assert_eq!(image[0x10000], 0xAA);
        assert!(image[..0x10000].iter().all(|b| *b == 0xFF));
    }

    #[test]
    fn applies_extended_segment_address() {
        let hex = ":020000021000EC\n:01000000AA55\n:00000001FF\n";
        assert_eq!(parse(hex).unwrap()[0x10000], 0xAA);
    }

    #[test]
    fn rejects_bad_checksum() {
        assert!(parse(":0400000001020304F3\n:00000001FF\n")
            .unwrap_err()
            .contains("checksum"));
    }

    #[test]
    fn rejects_missing_eof() {
        assert!(parse(":0400000001020304F2\n").is_err());
    }

    #[test]
    fn parses_bundled_firmwares() {
        for (name, hex) in [
            ("Blink.hex", include_str!("../../../utils/Blink.hex")),
            ("Melzi-v2.hex", include_str!("../../../utils/Melzi-v2.hex")),
            (
                "Marlin_Explorer_350.hex",
                include_str!("../../../utils/Marlin_Explorer_350.hex"),
            ),
        ] {
            let image = parse(hex).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(!image.is_empty(), "{name}");
        }
    }
}
