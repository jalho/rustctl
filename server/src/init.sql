/*
 * TABLES.
 */
CREATE TABLE users (
    id                   TEXT NOT NULL PRIMARY KEY,
    privileged           BOOLEAN NOT NULL
);
CREATE TABLE alt_ids (
    id                   TEXT NOT NULL PRIMARY KEY,
    steam_id             INTEGER NOT NULL,
    user_id              TEXT NOT NULL,
    created_at_utc       DATETIME NOT NULL,
    FOREIGN KEY(user_id) REFERENCES users(id)
);

/*
 * DATA.
 */
INSERT INTO users (
    id,
    privileged
) VALUES (
    '00000000-0000-0000-0000-000000000000',
    1
);
INSERT INTO alt_ids (
    id,
    steam_id,
    user_id,
    created_at_utc
) VALUES (
    '11111111-1111-1111-1111-111111111111',
    76561198135242017,
    '00000000-0000-0000-0000-000000000000',
    CURRENT_TIMESTAMP
);
