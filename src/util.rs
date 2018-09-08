/// Return the ith bit as a bool, where i is 0 indexed from the right
pub fn get_bit(data: u32, i: u8) -> bool {
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