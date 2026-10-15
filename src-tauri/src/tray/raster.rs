//! Draws the tray icon for Windows and Linux: a small calendar page with today's
//! day number. Light page, red header band and dark digits read well on both light
//! and dark taskbars, so no theme detection is needed.

use std::sync::OnceLock;

use ab_glyph::{Font, FontRef, PxScale, ScaleFont, point};
use ruz_core::format::Digits;
use tauri::image::Image;
use tiny_skia::{Color, FillRule, Mask, Paint, PathBuilder, Pixmap, Rect, Transform};

const SIZE: u32 = 32;
const BAND_HEIGHT: f32 = 9.0;
const CORNER: f32 = 5.0;

fn font() -> &'static FontRef<'static> {
    static FONT: OnceLock<FontRef<'static>> = OnceLock::new();
    FONT.get_or_init(|| {
        FontRef::try_from_slice(include_bytes!("../../assets/Vazirmatn-Bold.ttf"))
            .expect("bundled Vazirmatn-Bold.ttf is a valid font")
    })
}

fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish()
}

fn paint(r: u8, g: u8, b: u8) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color(Color::from_rgba8(r, g, b, 255));
    paint.anti_alias = true;
    paint
}

/// Coverage mask of `label`, centred in the page area below the band.
fn text_mask(label: &str, digits: Digits) -> Option<Mask> {
    let font = font();
    // Persian digits sit higher and are narrower than Latin ones at the same size.
    let px = if digits == Digits::Persian { 25.0 } else { 20.0 };
    let scaled = font.as_scaled(PxScale::from(px));
    let glyphs: Vec<_> = label.chars().map(|c| font.glyph_id(c)).collect();
    let width: f32 = glyphs.iter().map(|&g| scaled.h_advance(g)).sum();
    let mut x = (SIZE as f32 - width) / 2.0;

    let body_top = BAND_HEIGHT;
    let body_mid = body_top + (SIZE as f32 - body_top) / 2.0;
    let mut mask = Mask::new(SIZE, SIZE)?;
    let data = mask.data_mut();

    // Centre on the glyphs' actual ink, not the font's line metrics.
    let mut outlines = Vec::new();
    for &id in &glyphs {
        let glyph = id.with_scale_and_position(scaled.scale(), point(x, 0.0));
        x += scaled.h_advance(id);
        if let Some(outline) = font.outline_glyph(glyph) {
            outlines.push(outline);
        }
    }
    let ink_top = outlines.iter().map(|o| o.px_bounds().min.y).fold(f32::MAX, f32::min);
    let ink_bottom = outlines.iter().map(|o| o.px_bounds().max.y).fold(f32::MIN, f32::max);
    let dy = (body_mid - (ink_top + ink_bottom) / 2.0).round();

    for outline in outlines {
        let bounds = outline.px_bounds();
        outline.draw(|gx, gy, coverage| {
            let px = bounds.min.x as i32 + gx as i32;
            let py = (bounds.min.y + dy) as i32 + gy as i32;
            if (0..SIZE as i32).contains(&px) && (0..SIZE as i32).contains(&py) {
                let i = py as usize * SIZE as usize + px as usize;
                data[i] = data[i].max((coverage * 255.0) as u8);
            }
        });
    }
    Some(mask)
}

fn render(label: &str, digits: Digits) -> Option<Pixmap> {
    let mut pixmap = Pixmap::new(SIZE, SIZE)?;
    let page = rounded_rect(1.0, 1.0, SIZE as f32 - 2.0, SIZE as f32 - 2.0, CORNER)?;
    pixmap.fill_path(&page, &paint(250, 250, 248), FillRule::Winding, Transform::identity(), None);

    // Red band: the top of the page, clipped to its rounded corners.
    let mut page_mask = Mask::new(SIZE, SIZE)?;
    page_mask.fill_path(&page, FillRule::Winding, true, Transform::identity());
    let band = Rect::from_xywh(0.0, 0.0, SIZE as f32, BAND_HEIGHT)?;
    pixmap.fill_rect(band, &paint(209, 49, 42), Transform::identity(), Some(&page_mask));

    if !label.is_empty() {
        let mask = text_mask(label, digits)?;
        let all = Rect::from_xywh(0.0, 0.0, SIZE as f32, SIZE as f32)?;
        pixmap.fill_rect(all, &paint(28, 28, 30), Transform::identity(), Some(&mask));
    }
    Some(pixmap)
}

/// The tray icon for a day label such as "۲۳". An empty label draws a blank page.
pub fn day_icon(label: &str, digits: Digits) -> Image<'static> {
    let Some(pixmap) = render(label, digits) else {
        return Image::new_owned(vec![0; (SIZE * SIZE * 4) as usize], SIZE, SIZE);
    };
    // tiny-skia stores premultiplied colour; tray icons want straight RGBA.
    let rgba = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    Image::new_owned(rgba, SIZE, SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(image: &Image<'_>, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * SIZE + x) * 4) as usize;
        image.rgba()[i..i + 4].try_into().unwrap()
    }

    #[test]
    fn draws_band_page_and_digits() {
        let image = day_icon("۲۳", Digits::Persian);
        assert_eq!((image.width(), image.height()), (SIZE, SIZE));
        assert_eq!(pixel(&image, 16, 4), [209, 49, 42, 255], "red band");
        assert_eq!(pixel(&image, 0, 0)[3], 0, "transparent corner");
        let dark = (BAND_HEIGHT as u32..SIZE)
            .flat_map(|y| (0..SIZE).map(move |x| (x, y)))
            .filter(|&(x, y)| pixel(&image, x, y)[0] < 80)
            .count();
        assert!(dark > 20, "expected digit ink below the band, found {dark} dark pixels");
    }

    #[test]
    fn blank_page_has_no_ink() {
        let image = day_icon("", Digits::Persian);
        let dark = (BAND_HEIGHT as u32 + 1..SIZE - 2)
            .flat_map(|y| (2..SIZE - 2).map(move |x| (x, y)))
            .filter(|&(x, y)| pixel(&image, x, y)[0] < 80)
            .count();
        assert_eq!(dark, 0);
    }

    #[test]
    fn latin_digits_render_too() {
        let image = day_icon("31", Digits::Latin);
        let dark = (0..SIZE * SIZE).filter(|i| pixel(&image, i % SIZE, i / SIZE)[0] < 80).count();
        assert!(dark > 20);
    }
}
