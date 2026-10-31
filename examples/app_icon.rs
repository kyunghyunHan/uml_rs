//! Build native icon sizes from the repository's vector artwork.
fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).expect("iconset directory"));
    std::fs::create_dir_all(&out).unwrap();
    let tree = resvg::usvg::Tree::from_str(include_str!("../assets/icon.svg"), &Default::default())
        .unwrap();
    for size in [16, 32, 128, 256, 512] {
        for scale in [1, 2] {
            let px = size * scale;
            let mut pixmap = resvg::tiny_skia::Pixmap::new(px, px).unwrap();
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::from_scale(px as f32 / 1024., px as f32 / 1024.),
                &mut pixmap.as_mut(),
            );
            let suffix = if scale == 2 { "@2x" } else { "" };
            pixmap
                .save_png(out.join(format!("icon_{size}x{size}{suffix}.png")))
                .unwrap();
        }
    }
}
