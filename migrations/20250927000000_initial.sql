-- paws table is user <-> paw count <-> server mapping
CREATE TABLE paws (
    user_id BIGINT NOT NULL PRIMARY KEY,
    amount INTEGER DEFAULT 0
);
-- cooldowns tracks the last time a user performed some action
-- so that cooldowns can persist across bot restarts
CREATE TYPE cooldown_action AS ENUM ('paw', 'steal', 'gamble', 'spawn');
CREATE TABLE cooldowns (
    user_id BIGINT,
    action cooldown_action NOT NULL,
    expires timestamptz NOT NULL
);

-- tracks random paws that have spawned and whether they've been
-- claimed. this should be periodically cleaned up.
CREATE TABLE random_paws (
    message_id BIGINT NOT NULL PRIMARY KEY ,
    channel_id BIGINT NOT NULL,
    created timestamptz,
    claimed BOOLEAN NOT NULL DEFAULT false
);