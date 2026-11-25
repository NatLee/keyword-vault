CREATE TABLE users (
    id            INTEGER PRIMARY KEY,
    username      TEXT    NOT NULL UNIQUE COLLATE NOCASE,
    display_name  TEXT    NOT NULL,
    password_hash TEXT    NOT NULL,
    role          TEXT    NOT NULL DEFAULT 'member' CHECK (role IN ('admin', 'member')),
    disabled      INTEGER NOT NULL DEFAULT 0,
    created_at    TEXT    NOT NULL
);

-- 只存 session token 的 SHA-256
CREATE TABLE sessions (
    token_hash TEXT    PRIMARY KEY,
    user_id    INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    expires_at TEXT    NOT NULL,
    created_at TEXT    NOT NULL
);

CREATE TABLE user_groups (
    id         INTEGER PRIMARY KEY,
    name       TEXT    NOT NULL UNIQUE,
    created_at TEXT    NOT NULL
);

CREATE TABLE group_members (
    group_id INTEGER NOT NULL REFERENCES user_groups (id) ON DELETE CASCADE,
    user_id  INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    PRIMARY KEY (group_id, user_id)
);
CREATE INDEX group_members_user ON group_members (user_id);

CREATE TABLE keywords (
    id         INTEGER PRIMARY KEY,
    theme      TEXT    NOT NULL,
    -- 關鍵字語法。組合關鍵字存的是展開後的結果。
    expression TEXT    NOT NULL,
    -- 組合關鍵字的組合方式，例如 (#3|#5)!#8；一般關鍵字是 NULL。
    logic      TEXT,
    owner_id   INTEGER NOT NULL REFERENCES users (id),
    visibility TEXT    NOT NULL DEFAULT 'private' CHECK (visibility IN ('private', 'group', 'public')),
    created_at TEXT    NOT NULL,
    updated_at TEXT    NOT NULL
);
CREATE INDEX keywords_owner ON keywords (owner_id);
CREATE INDEX keywords_created ON keywords (created_at);

-- 組合關鍵字用到哪些關鍵字
CREATE TABLE keyword_refs (
    keyword_id INTEGER NOT NULL REFERENCES keywords (id) ON DELETE CASCADE,
    ref_id     INTEGER NOT NULL REFERENCES keywords (id),
    PRIMARY KEY (keyword_id, ref_id)
);
CREATE INDEX keyword_refs_ref ON keyword_refs (ref_id);

CREATE TABLE tags (
    id   INTEGER PRIMARY KEY,
    name TEXT    NOT NULL UNIQUE
);

CREATE TABLE keyword_tags (
    keyword_id INTEGER NOT NULL REFERENCES keywords (id) ON DELETE CASCADE,
    tag_id     INTEGER NOT NULL REFERENCES tags (id) ON DELETE CASCADE,
    PRIMARY KEY (keyword_id, tag_id)
);
CREATE INDEX keyword_tags_tag ON keyword_tags (tag_id);

CREATE TABLE comments (
    id         INTEGER PRIMARY KEY,
    keyword_id INTEGER NOT NULL REFERENCES keywords (id) ON DELETE CASCADE,
    user_id    INTEGER NOT NULL REFERENCES users (id),
    body       TEXT    NOT NULL,
    created_at TEXT    NOT NULL,
    updated_at TEXT    NOT NULL
);
CREATE INDEX comments_keyword ON comments (keyword_id);

CREATE TABLE favorites (
    user_id    INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    keyword_id INTEGER NOT NULL REFERENCES keywords (id) ON DELETE CASCADE,
    created_at TEXT    NOT NULL,
    PRIMARY KEY (user_id, keyword_id)
);

-- 每次語法改變時，記下改之前的內容
CREATE TABLE history (
    id         INTEGER PRIMARY KEY,
    keyword_id INTEGER NOT NULL REFERENCES keywords (id) ON DELETE CASCADE,
    expression TEXT    NOT NULL,
    user_id    INTEGER REFERENCES users (id) ON DELETE SET NULL,
    created_at TEXT    NOT NULL
);
CREATE INDEX history_keyword ON history (keyword_id);
