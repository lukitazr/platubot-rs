use super::super::graphics::{
    Fonts, NameAlignment, display_name, draw_avatar, label, one_line, unicode_name,
};
use crate::database::models::tournaments::{Match, Participant, Tournament};
use image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage, codecs::png::PngEncoder};
use imageproc::{
    drawing::{draw_filled_circle_mut, draw_filled_rect_mut},
    rect::Rect,
};
use std::{cmp::Ordering, collections::HashMap};

const WIDTH: u32 = 1080;
const TABLE_LEFT: i32 = 48;
const TABLE_WIDTH: u32 = 985;
const HEADER_TOP: i32 = 194;
const HEADER_HEIGHT: u32 = 50;
const ROW_TOP: i32 = 246;
const ROW_HEIGHT: u32 = 60;
const ROW_STEP: i32 = ROW_HEIGHT as i32 + 1;
const ROWS_PER_PAGE: usize = 30;
const FORM_LEFT: i32 = 834;

const BACKGROUND: Rgb<u8> = Rgb([8, 29, 24]);
const STRIPE: Rgb<u8> = Rgb([27, 57, 49]);
const ROW: Rgb<u8> = Rgb([20, 47, 40]);
const HEADER: Rgb<u8> = Rgb([36, 75, 62]);
const POINTS_BG: Rgb<u8> = Rgb([37, 73, 61]);
const POINTS: Rgb<u8> = Rgb([164, 247, 207]);
const WHITE: Rgb<u8> = Rgb([243, 249, 246]);
const MUTED: Rgb<u8> = Rgb([190, 208, 201]);
const ACCENT: Rgb<u8> = Rgb([117, 223, 255]);
const BORDER: Rgb<u8> = Rgb([49, 91, 80]);
const BADGE_OUTLINE: Rgb<u8> = Rgb([120, 166, 154]);
const DARK_TEXT: Rgb<u8> = Rgb([17, 41, 33]);

// Nombre, coordenada izquierda y ancho de las columnas de la variante con E y WO.
const COLUMNS: [(&str, i32, u32); 11] = [
    ("#", 58, 54),
    ("EQUIPO", 168, 200),
    ("PTS", 374, 70),
    ("PJ", 449, 59),
    ("GF:GC", 515, 95),
    ("DIF", 617, 66),
    ("G", 683, 36),
    ("E", 719, 34),
    ("P", 753, 34),
    ("WO", 790, 42),
    ("ÚLTIMOS 5", 833, 177),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResultChip {
    Win,
    Draw,
    Loss,
    Empty,
}

impl ResultChip {
    fn style(self) -> (&'static str, Rgb<u8>, Rgb<u8>) {
        match self {
            Self::Win => ("V", Rgb([158, 218, 156]), DARK_TEXT),
            Self::Draw => ("E", Rgb([255, 220, 131]), DARK_TEXT),
            Self::Loss => ("D", Rgb([255, 159, 149]), DARK_TEXT),
            Self::Empty => ("–", POINTS_BG, MUTED),
        }
    }
}

fn recent_form(user_id: i64, newest_matches: &[Match]) -> [ResultChip; 5] {
    let mut form = [ResultChip::Empty; 5];
    let results = newest_matches.iter().filter_map(|game| {
        let home = game.home_goals?;
        let away = game.away_goals?;
        let outcome = if game.home_id == user_id {
            home.cmp(&away)
        } else if game.away_id == user_id {
            away.cmp(&home)
        } else {
            return None;
        };
        Some(match outcome {
            Ordering::Greater => ResultChip::Win,
            Ordering::Equal => ResultChip::Draw,
            Ordering::Less => ResultChip::Loss,
        })
    });
    for (slot, result) in form.iter_mut().zip(results) {
        *slot = result;
    }
    form
}

pub(super) fn render(
    tournament: &Tournament,
    rows: &[(Participant, Vec<Match>)],
    avatars: &HashMap<i64, Vec<u8>>,
) -> Result<Vec<Vec<u8>>, crate::Error> {
    let fonts = Fonts::load()?;
    let pages = rows.len().div_ceil(ROWS_PER_PAGE);
    rows.chunks(ROWS_PER_PAGE)
        .enumerate()
        .map(|(page, rows)| {
            let canvas = render_page(&fonts, tournament, rows, avatars, page, pages);
            let mut png = Vec::new();
            PngEncoder::new(&mut png).write_image(
                canvas.as_raw(),
                canvas.width(),
                canvas.height(),
                ExtendedColorType::Rgb8,
            )?;
            Ok(png)
        })
        .collect()
}

fn render_page(
    fonts: &Fonts,
    tournament: &Tournament,
    rows: &[(Participant, Vec<Match>)],
    avatars: &HashMap<i64, Vec<u8>>,
    page: usize,
    pages: usize,
) -> RgbImage {
    let footer = ROW_TOP + ROW_STEP * rows.len() as i32 + 39;
    let mut canvas = RgbImage::from_pixel(WIDTH, (footer + 130) as u32, BACKGROUND);

    let title = one_line(&tournament.name).to_uppercase();
    let metadata = format!(
        "CLASIFICACIÓN   ·   ID {}   ·   {}   ·   {}",
        tournament.tournament_id,
        if tournament.oficial {
            "OFICIAL"
        } else {
            "AMISTOSO"
        },
        if tournament.finished {
            "FINALIZADO"
        } else {
            "EN CURSO"
        },
    );
    label(
        &mut canvas,
        &fonts.bold,
        Rect::at(TABLE_LEFT, 20).of_size(TABLE_WIDTH, 112),
        &title,
        72.0,
        WHITE,
        false,
    );
    label(
        &mut canvas,
        &fonts.regular,
        Rect::at(TABLE_LEFT, 134).of_size(TABLE_WIDTH, 40),
        &metadata,
        22.0,
        MUTED,
        false,
    );

    let header = Rect::at(TABLE_LEFT, HEADER_TOP).of_size(TABLE_WIDTH, HEADER_HEIGHT);
    draw_filled_rect_mut(&mut canvas, header, HEADER);
    for (index, (name, left, width)) in COLUMNS.iter().copied().enumerate() {
        label(
            &mut canvas,
            &fonts.bold,
            Rect::at(left, HEADER_TOP).of_size(width, HEADER_HEIGHT),
            name,
            19.0,
            if index == 2 { POINTS } else { MUTED },
            index == 1,
        );
    }

    let mut shows_champion = false;
    for (index, (participant, matches)) in rows.iter().enumerate() {
        let rank = page * ROWS_PER_PAGE + index + 1;
        let top = ROW_TOP + index as i32 * ROW_STEP;
        let champion = tournament.champion_id == Some(participant.user_id);
        shows_champion |= champion;
        draw_row(
            &mut canvas,
            fonts,
            participant,
            recent_form(participant.user_id, matches),
            rank,
            top,
            champion,
        );
        if let Some(png) = avatars.get(&participant.user_id) {
            draw_avatar(&mut canvas, 117, top as u32 + 10, 40, png);
        }
    }

    draw_filled_rect_mut(
        &mut canvas,
        Rect::at(TABLE_LEFT, footer).of_size(TABLE_WIDTH, 1),
        BORDER,
    );
    if shows_champion {
        draw_filled_rect_mut(
            &mut canvas,
            Rect::at(TABLE_LEFT, footer + 33).of_size(15, 15),
            ACCENT,
        );
        label(
            &mut canvas,
            &fonts.bold,
            Rect::at(77, footer + 21).of_size(180, 39),
            "Campeón",
            22.0,
            MUTED,
            true,
        );
    }
    for (chip, caption, left) in [
        (ResultChip::Win, "Victoria", 602),
        (ResultChip::Draw, "Empate", 736),
        (ResultChip::Loss, "Derrota", 875),
    ] {
        draw_chip(
            &mut canvas,
            fonts,
            Rect::at(left, footer + 26).of_size(29, 29),
            chip,
        );
        label(
            &mut canvas,
            &fonts.regular,
            Rect::at(left + 38, footer + 20).of_size(99, 41),
            caption,
            21.0,
            MUTED,
            true,
        );
    }
    if pages > 1 {
        label(
            &mut canvas,
            &fonts.regular,
            Rect::at(TABLE_LEFT, footer + 82).of_size(TABLE_WIDTH, 28),
            &format!("Página {} de {pages}", page + 1),
            20.0,
            MUTED,
            false,
        );
    }
    canvas
}

fn draw_row(
    canvas: &mut RgbImage,
    fonts: &Fonts,
    player: &Participant,
    form: [ResultChip; 5],
    rank: usize,
    top: i32,
    champion: bool,
) {
    let background = if rank % 2 == 1 { STRIPE } else { ROW };
    draw_filled_rect_mut(
        canvas,
        Rect::at(TABLE_LEFT, top).of_size(TABLE_WIDTH, ROW_HEIGHT),
        background,
    );
    draw_filled_rect_mut(
        canvas,
        Rect::at(374, top).of_size(70, ROW_HEIGHT),
        POINTS_BG,
    );
    if champion {
        draw_filled_rect_mut(
            canvas,
            Rect::at(TABLE_LEFT, top).of_size(4, ROW_HEIGHT),
            ACCENT,
        );
        draw_filled_rect_mut(
            canvas,
            Rect::at(59, top + 10).of_size(52, 40),
            Rgb([23, 63, 73]),
        );
    }
    draw_filled_circle_mut(canvas, (137, top + 30), 22, BADGE_OUTLINE);
    draw_filled_circle_mut(canvas, (137, top + 30), 20, background);

    let won = i128::from(player.pg);
    let drawn = i128::from(player.pe);
    let lost = i128::from(player.pp);
    let gd = i128::from(player.gf) - i128::from(player.gc);
    let values = [
        rank.to_string(),
        display_name(&player.username),
        (3 * won + drawn).to_string(),
        (won + drawn + lost).to_string(),
        format!("{}:{}", player.gf, player.gc),
        if gd > 0 {
            format!("+{gd}")
        } else {
            gd.to_string()
        },
        player.pg.to_string(),
        player.pe.to_string(),
        player.pp.to_string(),
        player.wo.to_string(),
    ];
    for (i, (value, (_, left, width))) in values.iter().zip(COLUMNS).enumerate() {
        let (font, size, color) = match i {
            0 => (&fonts.bold, 24.0, if champion { ACCENT } else { WHITE }),
            1 => (&fonts.bold, 25.0, WHITE),
            2 => (&fonts.bold, 32.0, POINTS),
            _ => (&fonts.regular, 24.0, MUTED),
        };
        if i == 1 {
            unicode_name(
                canvas,
                fonts,
                Rect::at(left, top).of_size(width, ROW_HEIGHT),
                value,
                WHITE,
                NameAlignment::Left,
            );
            continue;
        }
        label(
            canvas,
            font,
            Rect::at(left, top).of_size(width, ROW_HEIGHT),
            value,
            size,
            color,
            i == 1,
        );
    }
    for (index, chip) in form.into_iter().enumerate() {
        let chip_area = Rect::at(FORM_LEFT + index as i32 * 37, top + 14).of_size(29, 31);
        draw_chip(canvas, fonts, chip_area, chip);
    }
}

fn draw_chip(canvas: &mut RgbImage, fonts: &Fonts, bounds: Rect, chip: ResultChip) {
    let (mark, fill, ink) = chip.style();
    draw_filled_rect_mut(canvas, bounds, fill);
    label(canvas, &fonts.bold, bounds, mark, 19.0, ink, false);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_limits_graphemes_without_splitting_unicode() {
        assert_eq!(
            display_name("1234567890123456789e\u{301}xyz"),
            "1234567890123456789é"
        );
        assert_eq!(
            display_name("1234567890123456789👩‍💻fin"),
            "1234567890123456789👩‍💻"
        );
        assert_eq!(display_name("日本語玩家"), "日本語玩家");
    }

    fn game(home_id: i64, away_id: i64, home_goals: i64, away_goals: i64) -> Match {
        Match {
            fecha: 1,
            home_id,
            away_id,
            home_goals: Some(home_goals),
            away_goals: Some(away_goals),
        }
    }

    #[test]
    fn recent_form_keeps_newest_first_and_uses_the_players_side() {
        let mut unplayed = game(1, 2, 0, 0);
        unplayed.home_goals = None;
        let matches = [
            unplayed,
            game(2, 1, 1, 2),
            game(1, 2, 1, 1),
            game(1, 2, 0, 1),
        ];
        assert_eq!(
            recent_form(1, &matches),
            [
                ResultChip::Win,
                ResultChip::Draw,
                ResultChip::Loss,
                ResultChip::Empty,
                ResultChip::Empty
            ]
        );
    }

    #[test]
    fn render_encodes_full_rows_and_continues_on_a_second_png() {
        let tournament = Tournament {
            tournament_id: 10,
            guild_id: 1,
            name: "Copa Horizonte".into(),
            oficial: true,
            finished: true,
            champion_id: Some(31),
        };
        let names = ["Atlético Unión", "Vélez", "Central Córdoba", "Gimnasia (M)"];
        let rows: Vec<_> = (1..=31)
            .map(|user_id| {
                (
                    Participant {
                        user_id,
                        username: names[(user_id as usize - 1) % names.len()].into(),
                        pg: 10,
                        pe: 3,
                        pp: 2,
                        wo: 1,
                        gf: 24,
                        gc: 13,
                    },
                    vec![
                        game(user_id, 99, 2, 1),
                        game(99, user_id, 2, 2),
                        game(user_id, 99, 0, 1),
                    ],
                )
            })
            .collect();
        let mut avatar = Vec::new();
        let profile_color = Rgb([180, 50, 200]);
        let profile = RgbImage::from_pixel(64, 64, profile_color);
        PngEncoder::new(&mut avatar)
            .write_image(profile.as_raw(), 64, 64, ExtendedColorType::Rgb8)
            .unwrap();
        let pages = render(&tournament, &rows, &HashMap::from([(2, avatar)])).unwrap();
        assert_eq!(pages.len(), 2);
        let first = image::load_from_memory(&pages[0]).unwrap().to_rgb8();
        let last = image::load_from_memory(&pages[1]).unwrap().to_rgb8();
        assert_eq!(first.dimensions(), (1080, 2245));
        assert_eq!(last.width(), 1080);
        assert!(last.height() < first.height());

        // Check both outer edges, the one pixel separator and the winner's chip.
        assert_eq!(*first.get_pixel(48, 246), STRIPE);
        assert_eq!(*first.get_pixel(1032, 305), STRIPE);
        assert_eq!(*first.get_pixel(1033, 305), BACKGROUND);
        assert_eq!(*first.get_pixel(1032, 306), BACKGROUND);
        assert_eq!(*first.get_pixel(834, 260), ResultChip::Win.style().1);
        assert_eq!(*first.get_pixel(871, 260), ResultChip::Draw.style().1);
        assert_eq!(*first.get_pixel(908, 260), ResultChip::Loss.style().1);
        assert_eq!(*first.get_pixel(137, 276), STRIPE);
        assert_eq!(*first.get_pixel(137, 337), profile_color);
        assert_eq!(*first.get_pixel(117, 317), ROW);
        assert_eq!(*last.get_pixel(48, 246), ACCENT);
        assert_eq!(*first.get_pixel(1032, 2074), ROW);
        assert_eq!(*first.get_pixel(1032, 2075), BACKGROUND);
        assert_eq!(*last.get_pixel(1032, 246), STRIPE);
    }
}
