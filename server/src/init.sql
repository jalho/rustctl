/*
 * TABLES.
 */
CREATE TABLE steam_ids (
  id        TEXT PRIMARY KEY, -- UUIDv4
  steam_id  INTEGER NOT NULL  -- 17-digit unsigned integer
);
CREATE TABLE users (
  id          TEXT PRIMARY KEY, -- UUIDv4
  privileged  BOOLEAN NOT NULL
);

/*
 * DATA.
 */
INSERT INTO
    steam_ids(
        id,
        steam_id
    )
    VALUES(
        '00000000-0000-0000-0000-000000000000',
        76561198135242017
    );
INSERT INTO
    users(
        id,
        privileged
    )
    VALUES(
        '11111111-1111-1111-1111-111111111111',
        1
    );
