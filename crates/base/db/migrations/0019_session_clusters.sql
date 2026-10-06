-- The cluster contexts a session holds; an empty namespace is the whole cluster.
CREATE TABLE session_clusters (
    session_id  TEXT NOT NULL REFERENCES sessions(id)
                  ON DELETE CASCADE ON UPDATE CASCADE,
    context     TEXT NOT NULL,
    namespace   TEXT NOT NULL DEFAULT '',
    added_at    INTEGER NOT NULL,
    PRIMARY KEY (session_id, context, namespace)
);
