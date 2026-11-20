CREATE TABLE IF NOT EXISTS users (
    id BIGINT PRIMARY KEY,
    username TEXT NOT NULL
);
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS pg BIGINT NOT NULL DEFAULT 0 CHECK (pg >= 0),
    ADD COLUMN IF NOT EXISTS pe BIGINT NOT NULL DEFAULT 0 CHECK (pe >= 0),
    ADD COLUMN IF NOT EXISTS pp BIGINT NOT NULL DEFAULT 0 CHECK (pp >= 0),
    ADD COLUMN IF NOT EXISTS wo BIGINT NOT NULL DEFAULT 0 CHECK (wo >= 0),
    ADD COLUMN IF NOT EXISTS gf BIGINT NOT NULL DEFAULT 0 CHECK (gf >= 0),
    ADD COLUMN IF NOT EXISTS gc BIGINT NOT NULL DEFAULT 0 CHECK (gc >= 0),
    ADD COLUMN IF NOT EXISTS titulos_oficiales BIGINT NOT NULL DEFAULT 0 CHECK (titulos_oficiales BETWEEN 0 AND 4294967295),
    ADD COLUMN IF NOT EXISTS titulos_amistosos BIGINT NOT NULL DEFAULT 0 CHECK (titulos_amistosos BETWEEN 0 AND 4294967295);
CREATE TABLE IF NOT EXISTS guild_users (
    guild_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL REFERENCES users(id),
    level SMALLINT NOT NULL DEFAULT 0,
    experience BIGINT NOT NULL DEFAULT 0,
    last_xp_at TIMESTAMPTZ,
    PRIMARY KEY (guild_id, user_id)
);
CREATE TABLE tournaments (
    tournament_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    guild_id BIGINT NOT NULL,
    name TEXT NOT NULL,
    oficial BOOLEAN NOT NULL,
    finished BOOLEAN NOT NULL DEFAULT FALSE,
    champion_id BIGINT REFERENCES users(id)
);
CREATE TABLE tournament_participants (
    tournament_id BIGINT NOT NULL REFERENCES tournaments(tournament_id),
    user_id BIGINT NOT NULL REFERENCES users(id),
    pg BIGINT NOT NULL DEFAULT 0 CHECK (pg >= 0),
    pe BIGINT NOT NULL DEFAULT 0 CHECK (pe >= 0),
    pp BIGINT NOT NULL DEFAULT 0 CHECK (pp >= 0),
    gf BIGINT NOT NULL DEFAULT 0 CHECK (gf >= 0),
    gc BIGINT NOT NULL DEFAULT 0 CHECK (gc >= 0),
    PRIMARY KEY (tournament_id, user_id)
);
CREATE TABLE tournament_matches (
    match_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tournament_id BIGINT NOT NULL REFERENCES tournaments(tournament_id),
    fecha INTEGER NOT NULL CHECK (fecha > 0),
    home_id BIGINT NOT NULL,
    away_id BIGINT NOT NULL,
    home_goals BIGINT CHECK (home_goals >= 0),
    away_goals BIGINT CHECK (away_goals >= 0),
    CHECK (home_id <> away_id),
    CHECK ((home_goals IS NULL) = (away_goals IS NULL)),
    FOREIGN KEY (tournament_id, home_id) REFERENCES tournament_participants(tournament_id, user_id),
    FOREIGN KEY (tournament_id, away_id) REFERENCES tournament_participants(tournament_id, user_id)
);
