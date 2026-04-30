PRAGMA foreign_keys = ON;

CREATE TABLE accounts (
    id            TEXT    NOT NULL PRIMARY KEY,
    status        TEXT    NOT NULL CHECK (status IN ('Pending', 'Active', 'Suspended')),
    primary_email TEXT    NOT NULL,
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL,
    version       INTEGER NOT NULL
);

CREATE TABLE identities (
    provider       TEXT    NOT NULL,
    subject        TEXT    NOT NULL,
    account_id     TEXT    NOT NULL,
    email          TEXT    NOT NULL,
    email_verified INTEGER NOT NULL DEFAULT 0,
    linked_at      INTEGER NOT NULL,
    PRIMARY KEY (provider, subject)
);
CREATE INDEX idx_identities_account_id ON identities (account_id);

CREATE TABLE processed_events (
    event_id     TEXT    NOT NULL PRIMARY KEY,
    processed_at INTEGER NOT NULL
);

CREATE TABLE _atlas_revisions (
    version    TEXT    NOT NULL PRIMARY KEY,
    applied_at INTEGER NOT NULL
);
