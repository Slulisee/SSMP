use eframe::egui::Color32;
pub const NAMES: &[&str] = &[
    "Ocean", "Carbon", "Midnight", "Aurora", "Forest", "Sunset", "Sakura", "Lavender", "Frost",
    "Sand", "Nord", "Dracula", "Espresso", "Ruby", "Lagoon", "Contrast",
];
#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub panel: Color32,
    pub accent: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub border: Color32,
    pub light: bool,
}
fn c(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}
pub fn palette(name: &str) -> Palette {
    let (bg, panel, accent, text, muted, border, light) = match name {
        "Carbon" => (
            0x0c0c0c, 0x202020, 0xdedede, 0xf3f3f3, 0xb8b8b8, 0x424242, false,
        ),
        "Midnight" => (
            0x0c1020, 0x171e36, 0x858bff, 0xf0f1ff, 0xadb4d4, 0x303b5b, false,
        ),
        "Aurora" => (
            0x0b1420, 0x142434, 0x6de3c2, 0xecfff9, 0xa2c5c5, 0x294857, false,
        ),
        "Forest" => (
            0x091612, 0x14281f, 0x66db9b, 0xeffbf3, 0xaac9b5, 0x304e3c, false,
        ),
        "Sunset" => (
            0x1a101c, 0x302033, 0xff927f, 0xfff0f2, 0xd7b1c9, 0x5d3a54, false,
        ),
        "Sakura" => (
            0xfcf2f6, 0xfffafd, 0xb83d75, 0x422437, 0x805a72, 0xe6c8d8, true,
        ),
        "Lavender" => (
            0xf3f0fc, 0xfdfbff, 0x7851c9, 0x302344, 0x746583, 0xdcd2ed, true,
        ),
        "Frost" | "Daylight" => (
            0xeef4fa, 0xffffff, 0x0879cf, 0x182b41, 0x5f7590, 0xccdae8, true,
        ),
        "Sand" => (
            0xf6f0e6, 0xfffbf5, 0xa55830, 0x392d24, 0x7c6b5d, 0xdfd0bc, true,
        ),
        "Nord" => (
            0x232b38, 0x2d3949, 0x88c0d0, 0xedf1f5, 0xb4c3d4, 0x48596d, false,
        ),
        "Dracula" => (
            0x171622, 0x282436, 0xc59bff, 0xf8f5ff, 0xc0acd5, 0x4d3e63, false,
        ),
        "Espresso" => (
            0x17110f, 0x2b211c, 0xe0b17c, 0xfff2e5, 0xc9af98, 0x574135, false,
        ),
        "Ruby" => (
            0x170e15, 0x2a1823, 0xff6f91, 0xfff0f4, 0xd7aabc, 0x593247, false,
        ),
        "Lagoon" => (
            0x061718, 0x0d2a2b, 0x39d6d0, 0xe7fffb, 0x9accc7, 0x245353, false,
        ),
        "Contrast" | "OLED" => (
            0x050505, 0x111111, 0xffdd55, 0xffffff, 0xd4d4d4, 0x777777, false,
        ),
        _ => (
            0x080f18, 0x101c2b, 0x008cff, 0xeff5ff, 0xa0b9db, 0x22384e, false,
        ),
    };
    Palette {
        bg: c(bg),
        panel: c(panel),
        accent: c(accent),
        text: c(text),
        muted: c(muted),
        border: c(border),
        light,
    }
}
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    Color32::from_rgb(
        (a.r() as f32 * (1. - t) + b.r() as f32 * t) as u8,
        (a.g() as f32 * (1. - t) + b.g() as f32 * t) as u8,
        (a.b() as f32 * (1. - t) + b.b() as f32 * t) as u8,
    )
}
