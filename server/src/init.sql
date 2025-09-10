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
-- TODO!
