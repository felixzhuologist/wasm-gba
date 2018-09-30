//! The GBA IO Map is implemented by keeping two copies of each segment of the
//! IO map: a "parsed" version, which is a struct whose fields represent the
//! relevant values, and the raw data. Only keeping raw data would require
//! some bit manipulation logic each time we want to know a specific value,
//! but only keeping the parsed values would require converting to raw data
//! each time we want to read directly from memory.
//! By keeping both versions reading either type of value is straightforward, but
//! we need to update both on writes.
//! All raw data is handled directly by the Memory struct, and code for the
//! parsed data is handled in this module: generally each separate segment of
//! IO memory (graphics, sound, timers, DMA...) will have its own struct whose
//! fields are the parsed values and which provides a set_byte/hw/word method
//! that should be called when the corresponding area of raw memory is modified.

// TODO: currently set_halfword() and set_word() in each module
// is implemented in terms of set_byte(), but we might want to implement each
// to avoid doing redundant work
// TODO: should there be a trait for set_byte/halfword/word? would that make
// things easier without the use of boxed traits?

pub mod addrs;
pub mod graphics;
pub mod dma;