use super::super::graphics::{
    Fonts, NameAlignment, display_name, draw_avatar, label, one_line, unicode_name,
};
use super::Fixture;
use crate::{Error, database::models::tournaments::Match};
use image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage, codecs::png::PngEncoder};
use imageproc::{
    drawing::{draw_filled_circle_mut, draw_filled_rect_mut},
    rect::Rect,
};

const WIDTH: u32 = 1600;
const ROW_HEIGHT: u32 = 79;
const ROW_STEP: u32 = 86;
const BG: Rgb<u8> = Rgb([8, 29, 24]);
const ROW: Rgb<u8> = Rgb([20, 47, 40]);
const STRIPE: Rgb<u8> = Rgb([27, 57, 49]);
const LINE: Rgb<u8> = Rgb([49, 91, 80]);
const WHITE: Rgb<u8> = Rgb([243, 249, 246]);
const MUTED: Rgb<u8> = Rgb([190, 208, 201]);
const SKY: Rgb<u8> = Rgb([117, 223, 255]);
const MINT: Rgb<u8> = Rgb([164, 247, 207]);

pub(super) fn render(fixture: &Fixture, page: usize) -> Result<Vec<u8>, Error> {
    let fonts = Fonts::load()?;
    let (date, matches) = &fixture.dates[page];
    let columns = match_columns(matches.len());
    let per_column = matches.len() / columns;
    let top = 239;
    let footer = top
        + u32::try_from(per_column)?
            .checked_mul(ROW_STEP)
            .and_then(|height| height.checked_add(21))
            .ok_or("La fecha supera la altura del PNG")?;
    let height = footer
        .checked_add(79)
        .ok_or("La fecha supera la altura del PNG")?;
    let mut canvas = RgbImage::from_pixel(WIDTH, height, BG);

    label(
        &mut canvas,
        &fonts.bold,
        Rect::at(56, 66).of_size(1488, 73),
        &one_line(&fixture.tournament.name).to_uppercase(),
        61.0,
        WHITE,
        false,
    );
    let plate = Rect::at(710, 150).of_size(180, 43);
    draw_filled_rect_mut(&mut canvas, plate, SKY);
    let date_label = format!("FECHA {date}");
    label(
        &mut canvas,
        &fonts.bold,
        plate,
        &date_label,
        23.0,
        BG,
        false,
    );

    let (start, width, gutter) = if columns == 2 {
        (56, 730, 28)
    } else {
        (350, 900, 0)
    };
    for (index, game) in matches.iter().enumerate() {
        let column = index / per_column;
        let row = index % per_column;
        let x = start + column as i32 * (width as i32 + gutter);
        let y = top as i32 + row as i32 * ROW_STEP as i32;
        draw_match(
            &mut canvas,
            &fonts,
            fixture,
            game,
            Rect::at(x, y).of_size(width, ROW_HEIGHT),
            row,
        );
    }
    let footer = i32::try_from(footer)?;
    draw_filled_rect_mut(&mut canvas, Rect::at(56, footer).of_size(1488, 1), LINE);
    label(
        &mut canvas,
        &fonts.regular,
        Rect::at(560, footer + 12).of_size(480, 33),
        &format!("Página {} de {}", page + 1, fixture.dates.len()),
        18.0,
        MUTED,
        false,
    );

    let mut png = Vec::new();
    PngEncoder::new(&mut png).write_image(
        canvas.as_raw(),
        WIDTH,
        height,
        ExtendedColorType::Rgb8,
    )?;
    Ok(png)
}

pub(super) fn render_player(fixture: &Fixture, player: i64) -> Result<Vec<u8>, Error> {
    const MATCH_WIDTH: u32 = 590;
    const GAP: u32 = 24;
    const MARGIN: u32 = 40;
    const HEADING_TOP: u32 = 157;
    const MATCH_OFFSET: u32 = 51;

    let fonts = Fonts::load()?;
    let dates_per_column = fixture.dates.len() / match_columns(fixture.dates.len());
    let mut width = MARGIN;
    let mut height = 0;
    let mut positions = Vec::with_capacity(fixture.dates.len());
    for dates in fixture.dates.chunks(dates_per_column) {
        let mut top = HEADING_TOP;
        let mut column_width = 0;
        for (_, games) in dates {
            positions.push((width, top));
            let columns = match_columns(games.len());
            column_width = column_width.max(columns as u32 * (MATCH_WIDTH + GAP));
            let rows = u32::try_from(games.len() / columns)?;
            top = rows
                .checked_mul(ROW_STEP)
                .and_then(|rows| rows.checked_add(MATCH_OFFSET + GAP))
                .and_then(|block| top.checked_add(block))
                .ok_or("El fixture supera la altura del PNG")?;
        }
        height = height.max(top);
        width = width
            .checked_add(column_width)
            .ok_or("El fixture supera el ancho del PNG")?;
    }
    width = width
        .checked_add(MARGIN - GAP)
        .ok_or("El fixture supera el ancho del PNG")?;
    height = height
        .checked_add(8)
        .ok_or("El fixture supera la altura del PNG")?;
    i32::try_from(width)?;
    i32::try_from(height)?;
    let mut canvas = RgbImage::from_pixel(width, height, BG);
    label(
        &mut canvas,
        &fonts.bold,
        Rect::at(MARGIN as i32, 24).of_size(width - MARGIN * 2, 64),
        &one_line(&fixture.tournament.name).to_uppercase(),
        58.0,
        WHITE,
        false,
    );
    let player_plate = Rect::at((width as i32 - 220) / 2, 94).of_size(220, 43);
    draw_filled_rect_mut(&mut canvas, player_plate, SKY);
    unicode_name(
        &mut canvas,
        &fonts,
        player_plate,
        &name(fixture, player),
        BG,
        NameAlignment::Center,
    );

    for ((date, games), (left, top)) in fixture.dates.iter().zip(positions) {
        let columns = match_columns(games.len());
        let per_column = games.len() / columns;
        let block_width = columns as u32 * (MATCH_WIDTH + GAP) - GAP;
        let heading = Rect::at(left as i32, top as i32).of_size(block_width, 39);
        draw_filled_rect_mut(&mut canvas, heading, LINE);
        label(
            &mut canvas,
            &fonts.bold,
            heading,
            &format!("FECHA {date}"),
            23.0,
            WHITE,
            false,
        );
        for (index, game) in games.iter().enumerate() {
            let row = index % per_column;
            let x = left + (index / per_column) as u32 * (MATCH_WIDTH + GAP);
            let y = top + MATCH_OFFSET + row as u32 * ROW_STEP;
            draw_match(
                &mut canvas,
                &fonts,
                fixture,
                game,
                Rect::at(x as i32, y as i32).of_size(MATCH_WIDTH, ROW_HEIGHT),
                row,
            );
        }
    }
    let mut png = Vec::new();
    PngEncoder::new(&mut png).write_image(
        canvas.as_raw(),
        width,
        height,
        ExtendedColorType::Rgb8,
    )?;
    Ok(png)
}

fn match_columns(count: usize) -> usize {
    if count.is_multiple_of(2) { 2 } else { 1 }
}

fn draw_match(
    canvas: &mut RgbImage,
    fonts: &Fonts,
    fixture: &Fixture,
    game: &Match,
    row: Rect,
    index: usize,
) {
    let x = row.left();
    let y = row.top();
    let center = x + row.width() as i32 / 2;
    draw_filled_rect_mut(
        canvas,
        row,
        if index.is_multiple_of(2) { ROW } else { STRIPE },
    );
    let scores = game.home_goals.zip(game.away_goals);
    let score = scores.map_or_else(
        || "VS".to_owned(),
        |(home, away)| format!("{home} – {away}"),
    );
    draw_filled_rect_mut(
        canvas,
        Rect::at(center - 50, y + 14).of_size(100, 51),
        if scores.is_some() {
            LINE
        } else {
            Rgb([85, 119, 109])
        },
    );
    let plate = Rect::at(center - 48, y + 16).of_size(96, 47);
    draw_filled_rect_mut(
        canvas,
        plate,
        if scores.is_some() {
            Rgb([36, 75, 62])
        } else {
            ROW
        },
    );
    label(
        canvas,
        &fonts.bold,
        plate,
        &score,
        if scores.is_some() { 32.0 } else { 25.0 },
        if scores.is_some() { MINT } else { MUTED },
        false,
    );

    let name_width = row.width() / 2 - 141;
    for (id, avatar_x, name_x, home) in [
        (game.home_id, center - 115, x + 12, true),
        (game.away_id, center + 63, center + 123, false),
    ] {
        let selected = fixture.player == Some(id);
        let player = name(fixture, id);
        draw_filled_circle_mut(
            canvas,
            (avatar_x + 26, y + 39),
            27,
            if selected { SKY } else { LINE },
        );
        draw_filled_circle_mut(
            canvas,
            (avatar_x + 26, y + 39),
            25,
            if selected {
                Rgb([23, 63, 73])
            } else {
                Rgb([36, 66, 56])
            },
        );
        if let Some(png) = fixture.avatars.get(&id) {
            draw_avatar(canvas, (avatar_x + 2) as u32, (y + 15) as u32, 48, png);
        } else {
            let initials: String = player
                .chars()
                .take(2)
                .flat_map(char::to_uppercase)
                .collect();
            label(
                canvas,
                &fonts.bold,
                Rect::at(avatar_x, y + 13).of_size(52, 52),
                &initials,
                19.0,
                if selected { SKY } else { MUTED },
                false,
            );
        }
        let won = scores.is_some_and(|(h, a)| if home { h > a } else { a > h });
        unicode_name(
            canvas,
            fonts,
            Rect::at(name_x, y).of_size(name_width, ROW_HEIGHT),
            &player,
            if won { MINT } else { WHITE },
            if home {
                NameAlignment::Right
            } else {
                NameAlignment::Left
            },
        );
    }
}

fn name(fixture: &Fixture, id: i64) -> String {
    fixture
        .names
        .get(&id)
        .map(|name| display_name(name))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::models::tournaments::Tournament;

    fn game(fecha: i32, home_id: i64, away_id: i64) -> Match {
        Match {
            fecha,
            home_id,
            away_id,
            home_goals: None,
            away_goals: None,
        }
    }

    #[test]
    fn fixture_uses_each_dates_match_count_after_filtering() {
        let matches = || {
            let mut games = vec![game(18, 8, 9), game(17, 4, 5), game(17, 6, 7)];
            games.extend((0..31).map(|i| game(16, 100 + i, 200 + i)));
            games.push(game(16, 5, 8));
            games
        };
        let mut fixture = Fixture {
            tournament: Tournament {
                tournament_id: 10,
                guild_id: 1,
                name: "Clausura".into(),
                oficial: true,
                finished: false,
                champion_id: None,
            },
            dates: super::super::fechas(matches(), None),
            names: [(5, "日本語 jugador".into())].into(),
            avatars: Default::default(),
            player: None,
        };
        let general = image::load_from_memory(&render(&fixture, 0).unwrap())
            .unwrap()
            .to_rgb8();
        assert_eq!(*general.get_pixel(56, 1529), STRIPE);
        assert_eq!(*general.get_pixel(814, 1529), STRIPE);

        fixture.player = Some(5);
        fixture.dates = super::super::fechas(matches(), fixture.player);
        assert_eq!(
            fixture
                .dates
                .iter()
                .map(|(date, _)| *date)
                .collect::<Vec<_>>(),
            [16, 17]
        );
        let png = image::load_from_memory(&render_player(&fixture, 5).unwrap())
            .unwrap()
            .to_rgb8();
        assert_eq!(*png.get_pixel(40, 208), ROW);
        assert_eq!(*png.get_pixel(654, 208), ROW);
        assert_eq!(*png.get_pixel(640, 208), BG);
    }
}
