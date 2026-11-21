ALTER TABLE tournaments ADD COLUMN submit_channel_id BIGINT, ADD COLUMN review_channel_id BIGINT;
CREATE UNIQUE INDEX tournaments_submit_channel_unique ON tournaments(guild_id, submit_channel_id) WHERE submit_channel_id IS NOT NULL AND NOT finished;
CREATE TABLE match_submissions (
    submission_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tournament_id BIGINT NOT NULL REFERENCES tournaments(tournament_id),
    author_id BIGINT NOT NULL REFERENCES users(id),
    source_channel_id BIGINT NOT NULL,
    source_message_id BIGINT NOT NULL UNIQUE,
    match_id BIGINT REFERENCES tournament_matches(match_id),
    status TEXT NOT NULL DEFAULT 'choosing' CHECK (status IN ('choosing', 'sending', 'pending', 'approved', 'denied', 'ignored')),
    review_channel_id BIGINT,
    review_message_id BIGINT
);

