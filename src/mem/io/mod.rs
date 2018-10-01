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
//! fields are the parsed values and it is up to Memory to provide update each
//! struct if its raw data gets modified. The methods to update the struct belong
//! to Memory but are implemented in the submodules here

pub mod addrs;
pub mod graphics;
pub mod dma;
pub mod interrupt;