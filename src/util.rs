/// Return the ith bit as a bool, where i is 0 indexed from the right
pub fn get_bit(data: u32, i: u8) -> bool {
    ((data >> i) & 1) == 1
}

/// Return the ith bit as a bool, where i is 0 indexed from the right
pub fn get_bit_hw(data: u16, i: u8) -> bool {
    ((data >> i) & 1) == 1
}

/// Return the nibble starting at i (going leftwards) where i is 0 indexed from the right
pub fn get_nibble(data: u32, i: u8) -> u32 {
    (data >> i) & 0xF
}

//// Return the byte starting at i (going leftwards), where i is 0 indexed from the right
pub fn get_byte(data: u32, i: u8) -> u32 {
    (data >> i) & 0xFF
}

/// parse the following format into a float:
/// F E D C  B A 9 8  7 6 5 4  3 2 1 0
/// S I I I  I I I I  F F F F  F F F F
/// 0-7 (F) = Fraction
/// 8-E (I) = Integer
/// F   (S) = Sign bit
pub fn to_float_hw(raw: u16) -> f32 {
    let int = (raw >> 8) as i8 as f32;
    let frac = ((raw & 0xFF) as f32) / 256.0;
    int + frac
}

/// parse the following format into a float:
/// 27 26 25 24  23 22 21 20  19 18 17 16  15 14 13 12  11 10 9 8  7 6 5 4  3 2 1 0
/// S  I  I  I   I  I  I  I   I  I  I  I   I  I  I  I   I  I  I I  F F F F  F F F F
/// 0-7  (F) - Fraction
/// 8-26 (I) - Integer
/// 27   (S) - Sign bit
pub fn to_float_word(raw: u32) -> f32 {
    let mut int = (raw >> 8) & 0xFFFFF;
    if ((raw >> 27) & 1) == 1 {
        int |= 0xFFF0_0000; // sign extend
    }
    let frac = ((raw & 0xFF) as f32) / 256.0;
    (int as i32 as f32) + frac
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_float() {
        assert_eq!(to_float_hw(0x0A00), 10.0);
        assert_eq!(to_float_hw(0xFF00), -1.0);
        assert_eq!(to_float_hw(0x0180), 1.5);

        assert_eq!(to_float_word(0x00_000A_00), 10.0);
        assert_eq!(to_float_word(0xFF_FFFF_00), -1.0);
        assert_eq!(to_float_word(0x00_0002_80), 2.5);
    }
}
