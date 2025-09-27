-- paws table is user <-> paw count <-> server mapping
CREATE TABLE paws (
    user_id BIGINT NOT NULL PRIMARY KEY ,
    amount INTEGER DEFAULT 0
);
-- cooldowns tracks the last time a user performed some action
-- so that cooldowns can persist across bot restarts
CREATE TYPE cooldown_action AS ENUM ('paw', 'steal', 'gamble');
CREATE TABLE cooldowns (
    user_id BIGINT NOT NULL,
    action cooldown_action NOT NULL,
    expires timestamptz
);