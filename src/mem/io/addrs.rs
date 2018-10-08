// graphics
pub const GRAPHICS_START: u32 = 0x4000000;
pub const DISPCNT_LO: u32 = 0x04000000;
pub const DISPCNT_HI: u32 = 0x04000001;
pub const DISPSTAT_LO: u32 = 0x04000004;
pub const DISPSTAT_HI: u32 = 0x04000005;
pub const VCOUNT_LO: u32 = 0x4000006;
pub const BGCNT_START: u32 = 0x04000008;
pub const BGCNT_END: u32 = 0x0400000F;
pub const BG_OFFSET_START: u32 = 0x04000010;
pub const BG_OFFSET_END: u32 = 0x0400001F;
pub const BG_AFFINE_START: u32 = 0x04000020;
pub const BG_AFFINE_END: u32 = 0x0400003F;
pub const WIN_COORD_START: u32 = 0x4000040;
pub const WIN_COORD_END: u32 = 0x4000047;
pub const WIN_SETTINGS_START: u32 = 0x4000048;
pub const WIN_SETTINGS_END: u32 = 0x400004B;
pub const MOSAIC_LO: u32 = 0x400004C;
pub const MOSAIC_HI: u32 = 0x400004D;
pub const BLDCNT_LO: u32 = 0x4000050;
pub const BLDCNT_HI: u32 = 0x4000051;
pub const BLDALPHA_LO: u32 = 0x4000052;
pub const BLDALPHA_HI: u32 = 0x4000053;
pub const BLDY: u32 = 0x4000054;
pub const GRAPHICS_END: u32 = 0x4000055;

// DMA
pub const DMA_START: u32 = 0x40000B0;
pub const DMA_END: u32 = 0x40000DF;
pub const DMA_SAD: [u32; 4] = [0x40000B0, 0x40000BC, 0x40000C8, 0x40000D4];
pub const DMA_DAD: [u32; 4] = [0x40000B4, 0x40000C0, 0x40000CC, 0x40000D8];
pub const DMA_CNT: [u32; 4] = [0x40000BA, 0x40000C6, 0x40000D2, 0x40000DE];

// INTERRUPTS
pub const INT_START: u32 = 0x4000200;
pub const IE_LO: u32 = 0x4000200;
pub const IE_HI: u32 = 0x4000201;
pub const IF_LO: u32 = 0x4000202;
pub const IF_HI: u32 = 0x4000203;
pub const IME: u32 = 0x4000208;
pub const WSCNT_LO: u32 = 0x4000204;
pub const INT_END: u32 = 0x4000208;