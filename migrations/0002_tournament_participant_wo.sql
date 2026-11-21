ALTER TABLE tournament_participants
    ADD COLUMN wo BIGINT NOT NULL DEFAULT 0 CHECK (wo >= 0);
