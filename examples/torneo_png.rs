use std::{env, error::Error, fs, io, path::PathBuf};

use ab_glyph::FontArc;
use image::{ImageFormat, Rgb, RgbImage};
use imageproc::drawing::{draw_text_mut, text_size};

fn main() -> Result<(), Box<dyn Error>> {
    let font_path = env::args_os().nth(1).map(PathBuf::from).or_else(|| {
        env::var_os("WINDIR").map(|windows| PathBuf::from(windows).join("Fonts/arial.ttf"))
    });
    let font_path = font_path.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Indicá la ruta a una fuente TTF u OTF",
        )
    })?;
    let font = FontArc::try_from_vec(fs::read(font_path)?)?;

    let title = "Copa Horizonte";
    let scale = 76.0;
    let mut canvas = RgbImage::from_pixel(960, 480, Rgb([19, 35, 53]));
    let (width, height) = text_size(scale, &font, title);
    let x = (canvas.width().saturating_sub(width) / 2) as i32;
    let y = (canvas.height().saturating_sub(height) / 2) as i32;

    draw_text_mut(&mut canvas, Rgb([248, 250, 252]), x, y, scale, &font, title);
    canvas.save_with_format("torneo.png", ImageFormat::Png)?;
    println!("Imagen guardada en torneo.png");
    Ok(())
}
