use rand::{Rng, seq::SliceRandom};

pub type Fixture = (i64, i64);
pub type Schedule = Vec<Vec<Fixture>>;

// The circle method partitions all unordered pairs into dates with at most one
// match per participant. A missing slot represents a bye for odd-sized leagues.
pub fn draw(players: &[i64], rounds: usize) -> Option<Schedule> {
    if players.len() < 2 || rounds == 0 || players.len() > 64 || rounds > 20 {
        return None;
    }
    let mut rng = rand::thread_rng();
    for _ in 0..30 {
        let mut slots: Vec<Option<usize>> = (0..players.len()).map(Some).collect();
        slots.shuffle(&mut rng);
        if slots.len() % 2 != 0 {
            slots.push(None);
        }
        let mut dates = Vec::new();
        for _ in 0..slots.len() - 1 {
            let mut date = Vec::new();
            for i in 0..slots.len() / 2 {
                if let (Some(a), Some(b)) = (slots[i], slots[slots.len() - 1 - i]) {
                    date.push((a, b));
                }
            }
            date.shuffle(&mut rng);
            dates.push(date);
            let last = slots.pop().unwrap();
            slots.insert(1, last);
        }
        dates.shuffle(&mut rng);
        let mut oriented = vec![Vec::new(); dates.len()];
        let mut states = vec![State::default(); players.len()];
        let mut budget = 500_000;
        if assign(
            &dates,
            &mut oriented,
            &mut states,
            0,
            0,
            rounds,
            &mut budget,
            &mut rng,
        ) {
            let mut output = Vec::new();
            for round in 0..rounds {
                let reversed = round % 2 == 1;
                let iter: Box<dyn Iterator<Item = &Vec<(usize, usize)>>> = if reversed {
                    Box::new(oriented.iter().rev())
                } else {
                    Box::new(oriented.iter())
                };
                for date in iter {
                    output.push(
                        date.iter()
                            .map(|&(a, b)| {
                                if reversed {
                                    (players[b], players[a])
                                } else {
                                    (players[a], players[b])
                                }
                            })
                            .collect(),
                    );
                }
            }
            return Some(output);
        }
    }
    None
}

#[derive(Clone, Copy, Default)]
struct State {
    home: usize,
    away: usize,
    streak: i8,
}

fn assign(
    dates: &[Vec<(usize, usize)>],
    oriented: &mut [Vec<(usize, usize)>],
    states: &mut [State],
    date: usize,
    game: usize,
    rounds: usize,
    budget: &mut usize,
    rng: &mut impl Rng,
) -> bool {
    if *budget == 0 {
        return false;
    }
    *budget -= 1;
    if date == dates.len() {
        if states.iter().any(|s| s.home.abs_diff(s.away) > 1) {
            return false;
        }
        return validate_repeated(oriented, states.len(), rounds);
    }
    if game == dates[date].len() {
        // A bye breaks a run of consecutive dates.
        let mut absent = Vec::new();
        for (p, state) in states.iter_mut().enumerate() {
            if !dates[date].iter().any(|&(a, b)| a == p || b == p) {
                absent.push((p, state.streak));
                state.streak = 0;
            }
        }
        let ok = assign(dates, oriented, states, date + 1, 0, rounds, budget, rng);
        for (p, streak) in absent {
            states[p].streak = streak;
        }
        return ok;
    }
    let (a, b) = dates[date][game];
    let choices = if rng.gen_bool(0.5) {
        [(a, b), (b, a)]
    } else {
        [(b, a), (a, b)]
    };
    let max_side = (states.len() - 1).div_ceil(2);
    for (home, away) in choices {
        let h = states[home];
        let v = states[away];
        if h.streak == 2 || v.streak == -2 || h.home == max_side || v.away == max_side {
            continue;
        }
        states[home].home += 1;
        states[home].streak = if h.streak > 0 { h.streak + 1 } else { 1 };
        states[away].away += 1;
        states[away].streak = if v.streak < 0 { v.streak - 1 } else { -1 };
        oriented[date].push((home, away));
        if assign(dates, oriented, states, date, game + 1, rounds, budget, rng) {
            return true;
        }
        oriented[date].pop();
        states[home] = h;
        states[away] = v;
    }
    false
}

fn validate_repeated(dates: &[Vec<(usize, usize)>], count: usize, rounds: usize) -> bool {
    let mut streaks = vec![0i8; count];
    for round in 0..rounds {
        for i in 0..dates.len() {
            let reverse = round % 2 == 1;
            let date = &dates[if reverse { dates.len() - 1 - i } else { i }];
            let mut playing = vec![false; count];
            for &(a, b) in date {
                let (home, away) = if reverse { (b, a) } else { (a, b) };
                playing[home] = true;
                playing[away] = true;
                streaks[home] = if streaks[home] > 0 {
                    streaks[home] + 1
                } else {
                    1
                };
                streaks[away] = if streaks[away] < 0 {
                    streaks[away] - 1
                } else {
                    -1
                };
                if streaks[home] > 2 || streaks[away] < -2 {
                    return false;
                }
            }
            for (p, state) in streaks.iter_mut().enumerate() {
                if !playing[p] {
                    *state = 0;
                }
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn round_robin_invariants_and_mirrored_rounds() {
        for n in 2..=16 {
            for rounds in 1..=3 {
                for _ in 0..3 {
                    let players: Vec<i64> = (1..=n).collect();
                    let dates = draw(&players, rounds).expect("schedule exists");
                    let per_round = if n % 2 == 0 { n - 1 } else { n } as usize;
                    assert_eq!(dates.len(), per_round * rounds);
                    let mut counts = HashMap::<i64, (usize, usize, i8)>::new();
                    let mut pairs = HashMap::new();
                    for (i, date) in dates.iter().enumerate() {
                        assert_eq!(date.len(), (n / 2) as usize);
                        let mut used = HashSet::new();
                        for &(a, b) in date {
                            assert!(used.insert(a) && used.insert(b));
                            *pairs
                                .entry((i / per_round, a.min(b), a.max(b)))
                                .or_insert(0) += 1;
                            let h = counts.entry(a).or_default();
                            h.0 += 1;
                            h.2 = if h.2 > 0 { h.2 + 1 } else { 1 };
                            assert!(h.2 <= 2);
                            let v = counts.entry(b).or_default();
                            v.1 += 1;
                            v.2 = if v.2 < 0 { v.2 - 1 } else { -1 };
                            assert!(v.2 >= -2);
                        }
                        for p in &players {
                            if !used.contains(p) {
                                counts.entry(*p).or_default().2 = 0;
                            }
                        }
                    }
                    assert_eq!(pairs.len(), rounds * (n * (n - 1) / 2) as usize);
                    assert!(pairs.values().all(|&v| v == 1));
                    for (h, a, _) in counts.values() {
                        assert!(h.abs_diff(*a) <= 1);
                        if rounds % 2 == 0 || n % 2 == 1 {
                            assert_eq!(h, a);
                        }
                    }
                    if rounds > 1 {
                        for i in 0..per_round {
                            let expected: Vec<_> = dates[per_round - 1 - i]
                                .iter()
                                .map(|&(a, b)| (b, a))
                                .collect();
                            assert_eq!(dates[per_round + i], expected);
                        }
                    }
                }
            }
        }
    }
}
