-- The lists a session pins: a kind, and the contexts and namespaces it reads.
CREATE TABLE session_pins (
    session_id    TEXT NOT NULL REFERENCES sessions(id)
                    ON DELETE CASCADE ON UPDATE CASCADE,
    kind_group    TEXT NOT NULL,
    kind_version  TEXT NOT NULL,
    kind          TEXT NOT NULL,
    plural        TEXT NOT NULL,
    namespaced    INTEGER NOT NULL,
    -- One `context<TAB>namespace` a line, sorted; an empty namespace is the whole cluster.
    scope         TEXT NOT NULL,
    pinned_at     INTEGER NOT NULL,
    PRIMARY KEY (session_id, kind_group, kind, scope)
);
