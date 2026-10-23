use crate::scene::{self, ExportOptions, Scene};
use std::{path::Path, sync::Arc};
fn tree(svg: &str) -> Result<resvg::usvg::Tree, String> {
    let mut options = resvg::usvg::Options::default();
    let mut db = resvg::usvg::fontdb::Database::new();
    db.load_font_data(scene::FONT.to_vec());
    options.fontdb = Arc::new(db);
    resvg::usvg::Tree::from_str(svg, &options).map_err(|e| format!("SVG 렌더링 실패: {e}"))
}
pub fn png(scene: &Scene, path: &Path, options: &ExportOptions) -> Result<(u32, u32), String> {
    let (svg, w, h) = scene::svg(scene, options)?;
    let tree = tree(&svg)?;
    let mut pixels = resvg::tiny_skia::Pixmap::new(w, h)
        .ok_or("이미지 메모리를 확보할 수 없습니다. 배율을 낮추세요")?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixels.as_mut(),
    );
    crate::storage::atomic_write(path, &pixels.encode_png().map_err(|e| e.to_string())?)?;
    Ok((w, h))
}
pub fn svg(scene: &Scene, path: &Path, options: &ExportOptions) -> Result<(u32, u32), String> {
    let (svg, w, h) = scene::svg(scene, options)?;
    let outlined = tree(&svg)?.to_string(&resvg::usvg::WriteOptions::default());
    crate::storage::atomic_write(path, outlined.as_bytes())?;
    Ok((w, h))
}
pub fn preview(svg: &str, w: u32, h: u32) -> Result<eframe::egui::ColorImage, String> {
    let factor = (600. / w as f32).min(340. / h as f32).min(1.);
    let width = (w as f32 * factor).ceil().max(1.) as u32;
    let height = (h as f32 * factor).ceil().max(1.) as u32;
    let mut pixels =
        resvg::tiny_skia::Pixmap::new(width, height).ok_or("미리보기 메모리 확보 실패")?;
    resvg::render(
        &tree(svg)?,
        resvg::tiny_skia::Transform::from_scale(factor, factor),
        &mut pixels.as_mut(),
    );
    Ok(eframe::egui::ColorImage::from_rgba_premultiplied(
        [width as usize, height as usize],
        pixels.data(),
    ))
}
