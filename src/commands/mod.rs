mod diagnostics;
mod tournaments;
mod users;

use crate::{Data, Error};

pub(crate) struct CommandFactory(pub(crate) fn() -> poise::Command<Data, Error>);

inventory::collect!(CommandFactory);

pub(crate) fn all() -> Vec<poise::Command<Data, Error>> {
    let mut commands: Vec<_> = inventory::iter::<CommandFactory>
        .into_iter()
        .map(|factory| (factory.0)())
        .collect();
    commands.sort_by(|left, right| left.name.cmp(&right.name));
    commands
}

#[cfg(test)]
mod tests {
    #[test]
    fn collects_root_commands_and_preserves_tournament_subcommands() {
        let commands = super::all();

        for name in [
            "ping",
            "db_ping",
            "stress_test",
            "user_info",
            "torneo",
            "fixture",
            "tabla",
        ] {
            assert_eq!(
                commands
                    .iter()
                    .filter(|command| command.name == name)
                    .count(),
                1
            );
        }

        let torneo = commands
            .iter()
            .find(|command| command.name == "torneo")
            .unwrap();
        for name in ["crear", "fixture", "tabla"] {
            assert!(
                torneo
                    .subcommands
                    .iter()
                    .any(|command| command.name == name)
            );
        }
        assert!(!commands.iter().any(|command| command.name == "crear"));
    }
}
