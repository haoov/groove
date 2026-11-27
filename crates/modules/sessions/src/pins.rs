//! The lists a session pins, each a kind and the contexts and namespaces it reads.

use groove_types::{Attached, KubeKind, Pin, SessionId, Timestamp};

use crate::{Result, Store};

impl Store {
    /// Pinning the same list again changes nothing.
    pub async fn pin(&self, id: &SessionId, pin: &Pin, now: Timestamp) -> Result<()> {
        sqlx::query(
            "INSERT INTO session_pins
                (session_id, kind_group, kind_version, kind, plural, namespaced, scope, pinned_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(session_id, kind_group, kind, scope) DO NOTHING",
        )
        .bind(id.as_str())
        .bind(&pin.kind.group)
        .bind(&pin.kind.version)
        .bind(&pin.kind.kind)
        .bind(&pin.kind.plural)
        .bind(pin.kind.namespaced)
        .bind(scope_text(&pin.scope))
        .bind(now.seconds())
        .execute(self.db.pool())
        .await?;
        Ok(())
    }

    pub async fn unpin(&self, id: &SessionId, pin: &Pin) -> Result<()> {
        sqlx::query(
            "DELETE FROM session_pins
             WHERE session_id = ? AND kind_group = ? AND kind = ? AND scope = ?",
        )
        .bind(id.as_str())
        .bind(&pin.kind.group)
        .bind(&pin.kind.kind)
        .bind(scope_text(&pin.scope))
        .execute(self.db.pool())
        .await?;
        Ok(())
    }

    /// In the order they were pinned.
    pub async fn pins_of(&self, id: &SessionId) -> Result<Vec<Pin>> {
        let rows: Vec<PinRow> = sqlx::query_as(
            "SELECT kind_group, kind_version, kind, plural, namespaced, scope FROM session_pins
             WHERE session_id = ? ORDER BY pinned_at, kind_group, kind, scope",
        )
        .bind(id.as_str())
        .fetch_all(self.db.pool())
        .await?;
        Ok(rows.into_iter().map(pinned).collect())
    }
}

type PinRow = (String, String, String, String, bool, String);

fn pinned((group, version, kind, plural, namespaced, scope): PinRow) -> Pin {
    let kind = KubeKind {
        group,
        version,
        kind,
        plural,
        namespaced,
        watchable: true,
        short_names: Vec::new(),
    };
    Pin::new(&kind, scope_of(&scope))
}

/// One `context<TAB>namespace` a line; an empty namespace is the whole cluster.
fn scope_text(scope: &[Attached]) -> String {
    let line = |one: &Attached| {
        format!(
            "{}\t{}",
            one.context,
            one.namespace.as_deref().unwrap_or_default()
        )
    };
    scope.iter().map(line).collect::<Vec<_>>().join("\n")
}

fn scope_of(text: &str) -> Vec<Attached> {
    text.lines()
        .filter_map(|line| line.split_once('\t'))
        .map(|(context, namespace)| Attached {
            context: context.to_string(),
            namespace: (!namespace.is_empty()).then(|| namespace.to_string()),
        })
        .collect()
}
