/*
 * TABLES.
 */
CREATE TABLE users (
    id                   TEXT PRIMARY KEY, -- UUIDv4
    privileged           BOOLEAN NOT NULL
);
CREATE TABLE alt_ids (
    id                   TEXT PRIMARY KEY, -- UUIDv4
    steam_id             INTEGER NOT NULL, -- 17-digit unsigned integer
    user_id              TEXT, -- UUIDv4
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
    user_id
) VALUES (
    '11111111-1111-1111-1111-111111111111',
    76561198135242017,
    '00000000-0000-0000-0000-000000000000'
);
