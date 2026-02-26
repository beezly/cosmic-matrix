use cosmic::iced::Color;

pub fn sender_color(user_id: &str) -> Color {
    const PALETTE: [(f32, f32, f32); 8] = [
        (0.306, 0.663, 0.863), // sky blue
        (0.863, 0.412, 0.353), // coral
        (0.396, 0.729, 0.510), // sage
        (0.647, 0.518, 0.851), // lavender
        (0.831, 0.627, 0.235), // amber
        (0.259, 0.741, 0.741), // teal
        (0.835, 0.471, 0.647), // rose
        (0.569, 0.729, 0.255), // lime
    ];
    let idx = hash_user_id(user_id) % PALETTE.len();
    let (r, g, b) = PALETTE[idx];
    Color::from_rgb(r, g, b)
}

fn hash_user_id(user_id: &str) -> usize {
    let mut hash: u32 = 2_166_136_261u32; // FNV-1a offset basis
    for byte in user_id.bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(16_777_619); // FNV prime
    }
    hash as usize
}
