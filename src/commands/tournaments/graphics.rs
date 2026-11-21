use ab_glyph::{Font, FontRef};
use image::{Rgb, RgbImage};
use imageproc::{
    drawing::{draw_text_mut, text_size},
    rect::Rect,
};
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

pub(super) struct Fonts {
    pub(super) regular: FontRef<'static>,
    pub(super) bold: FontRef<'static>,
    unicode: FontRef<'static>,
    cjk: FontRef<'static>,
}

impl Fonts {
    pub(super) fn load() -> Result<Self, crate::Error> {
        Ok(Self {
            regular: FontRef::try_from_slice(include_bytes!(
                "../../../assets/fonts/Lato-Regular.ttf"
            ))?,
            bold: FontRef::try_from_slice(include_bytes!("../../../assets/fonts/Lato-Bold.ttf"))?,
            unicode: FontRef::try_from_slice(include_bytes!("../../../assets/fonts/NotoSans.ttf"))?,
            cjk: FontRef::try_from_slice(include_bytes!(
                "../../../assets/fonts/NotoSansCJK-Regular.otf"
            ))?,
        })
    }
}

pub(super) fn draw_avatar(canvas: &mut RgbImage, left: u32, top: u32, diameter: u32, png: &[u8]) {
    let Ok(image) = image::load_from_memory_with_format(png, image::ImageFormat::Png) else {
        return;
    };
    let avatar = image
        .resize_to_fill(diameter, diameter, image::imageops::FilterType::Triangle)
        .to_rgba8();
    // Keep the original ring and composite only pixels inside the circular mask.
    let radius = diameter as f32 / 2.0;
    for (x, y, pixel) in avatar.enumerate_pixels() {
        let dx = x as f32 + 0.5 - radius;
        let dy = y as f32 + 0.5 - radius;
        let coverage = (radius - dx.hypot(dy)).clamp(0.0, 1.0);
        let alpha = coverage * f32::from(pixel[3]) / 255.0;
        let dest = canvas.get_pixel_mut(left + x, top + y);
        for channel in 0..3 {
            dest[channel] = (f32::from(pixel[channel]) * alpha
                + f32::from(dest[channel]) * (1.0 - alpha))
                .round() as u8;
        }
    }
}

pub(super) fn label(
    canvas: &mut RgbImage,
    font: &FontRef<'_>,
    bounds: Rect,
    text: &str,
    size: f32,
    ink: Rgb<u8>,
    left_align: bool,
) {
    let (width, _) = text_size(size, font, text);
    let budget = bounds.width().saturating_sub(2);
    let scale = if width > budget {
        size * budget as f32 / width as f32
    } else {
        size
    };
    let (width, height) = text_size(scale, font, text);
    let x = if left_align {
        bounds.left()
    } else {
        bounds.left() + (bounds.width().saturating_sub(width) / 2) as i32
    };
    let y = bounds.top() + (bounds.height().saturating_sub(height) / 2) as i32;
    draw_text_mut(canvas, ink, x, y, scale, font, text);
}

pub(super) fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn display_name(text: &str) -> String {
    let normalized: String = one_line(text).nfc().collect();
    normalized.graphemes(true).take(20).collect()
}

pub(super) enum NameAlignment {
    Left,
    Center,
    Right,
}

pub(super) fn unicode_name(
    canvas: &mut RgbImage,
    fonts: &Fonts,
    bounds: Rect,
    text: &str,
    color: Rgb<u8>,
    alignment: NameAlignment,
) {
    let runs: Vec<_> = text
        .graphemes(true)
        .map(|grapheme| {
            let font = [&fonts.bold, &fonts.unicode, &fonts.cjk]
                .into_iter()
                .find(|font| grapheme.chars().all(|ch| font.glyph_id(ch).0 != 0))
                .unwrap_or(&fonts.unicode);
            (font, grapheme)
        })
        .collect();
    let width: u32 = runs
        .iter()
        .map(|(font, run)| text_size(25.0, *font, run).0.saturating_sub(1))
        .sum();
    let size = if width > bounds.width() - 2 {
        25.0 * (bounds.width() - 2) as f32 / width as f32
    } else {
        25.0
    };
    let fitted_width: u32 = runs
        .iter()
        .map(|(font, run)| text_size(size, *font, run).0.saturating_sub(1))
        .sum();
    let remaining = bounds.width().saturating_sub(fitted_width) as i32;
    let mut x = bounds.left()
        + match alignment {
            NameAlignment::Left => 0,
            NameAlignment::Center => remaining / 2,
            NameAlignment::Right => remaining,
        };
    for (font, run) in runs {
        let (width, height) = text_size(size, font, run);
        let y = bounds.top() + bounds.height().saturating_sub(height) as i32 / 2;
        draw_text_mut(canvas, color, x, y, size, font, run);
        x += width.saturating_sub(1) as i32;
    }
}
