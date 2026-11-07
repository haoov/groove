//! An Application's resource health from Argo CD's API, through the hub's API server: read once, then streamed.

use std::collections::BTreeMap;
use std::path::PathBuf;

use futures_util::StreamExt;
use groove_kube::{Client, Proxied};
use groove_types::ResourceHealth;

use crate::Stop;
use crate::retry::Backoff;

/// Each resource's health as last read, or what stopped the read.
pub type Treed = Result<Vec<ResourceHealth>, String>;

/// The Application, by the hub context it stands on, its namespace and its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgoApp {
    pub hub: String,
    pub namespace: String,
    pub name: String,
}

/// Follows `app`'s tree until `stop`; each tree, or each refusal, goes through `send`.
pub fn argo_tree(
    (paths, home): (Vec<PathBuf>, PathBuf),
    app: ArgoApp,
    stop: &Stop,
    send: impl Fn(Treed) + Send + Sync + 'static,
) -> impl std::future::Future<Output = ()> + Send + 'static {
    let mut heard = stop.subscribe();
    async move {
        let mut backoff = Backoff::new();
        loop {
            let ended = tokio::select! {
                _ = heard.stopped() => return,
                ended = cycle((&paths, &home), &app, &send, &mut backoff) => ended,
            };
            if let Err(why) = ended {
                send(Err(why));
            }
            if !heard.rest(backoff.grow()).await {
                return;
            }
        }
    }
}

/// One read of the tree, then its stream until it ends.
async fn cycle(
    (paths, home): (&[PathBuf], &PathBuf),
    app: &ArgoApp,
    send: &impl Fn(Treed),
    backoff: &mut Backoff,
) -> Result<(), String> {
    let opened = Opened::of((paths, home), app).await?;
    send(Ok(opened.read(app).await?));
    backoff.reset();
    let streamed = groove_argocd::tree_path(&app.namespace, &app.name, true);
    let headers = [("Cookie", opened.cookie.as_str())];
    let lines = opened
        .client
        .proxy_lines(&opened.to, &streamed, &headers)
        .await;
    let mut lines = std::pin::pin!(lines.map_err(|e| e.to_string())?);
    while let Some(line) = lines.next().await {
        let line = line.map_err(|e| e.to_string())?;
        match groove_argocd::streamed(&line) {
            Some(Ok(tree)) => send(Ok(tree)),
            Some(Err(why)) => return Err(refused(&opened.server, &why)),
            None => {}
        }
    }
    Ok(())
}

/// `app`'s tree read once.
pub async fn argo_tree_once((paths, home): (Vec<PathBuf>, PathBuf), app: ArgoApp) -> Treed {
    let opened = Opened::of((&paths, &home), &app).await?;
    opened.read(&app).await
}

/// The hub's client, argocd-server behind its API server, and the CLI's login.
pub(crate) struct Opened {
    client: Client,
    to: Proxied,
    server: String,
    cookie: String,
}

impl Opened {
    pub(crate) async fn of(
        (paths, home): (&[PathBuf], &PathBuf),
        app: &ArgoApp,
    ) -> Result<Self, String> {
        let (server, token) = token(home).await?;
        let client = Client::shared(paths, &app.hub).await;
        let client = client.map_err(|e| e.to_string())?;
        let to = reach(&client).await?;
        let cookie = token.cookie();
        Ok(Self {
            client,
            to,
            server,
            cookie,
        })
    }

    /// The body `path` answers on argocd-server; `Err` says how to sign in again when refused.
    pub(crate) async fn get(&self, path: &str) -> Result<String, String> {
        let headers = [("Cookie", self.cookie.as_str())];
        let body = self.client.proxy_get(&self.to, path, &headers).await;
        body.map_err(|e| refused(&self.server, &e.to_string()))
    }

    pub(crate) fn refused(&self, why: &str) -> String {
        refused(&self.server, why)
    }

    async fn read(&self, app: &ArgoApp) -> Treed {
        let once = groove_argocd::tree_path(&app.namespace, &app.name, false);
        let headers = [("Cookie", self.cookie.as_str())];
        let body = self.client.proxy_get(&self.to, &once, &headers).await;
        let body = body.map_err(|e| refused(&self.server, &e.to_string()))?;
        groove_argocd::health(&body).map_err(|why| refused(&self.server, &why))
    }
}

/// The CLI's server and token, read again each time so a new login is picked up.
async fn token(home: &PathBuf) -> Result<(String, groove_argocd::Token), String> {
    let path = groove_argocd::config_path(home);
    let config = tokio::fs::read_to_string(&path).await.unwrap_or_default();
    groove_argocd::token_of(&config)
        .ok_or_else(|| "no Argo CD login: run argocd login <server> --sso".to_string())
}

/// What Argo CD said, with how to sign in again when it refused the session.
fn refused(server: &str, why: &str) -> String {
    let session = [
        "no session information",
        "token is expired",
        "invalid session",
    ];
    match session.iter().any(|one| why.contains(one)) {
        true => format!("Argo CD refused the login ({why}): run argocd login {server} --sso"),
        false => why.to_string(),
    }
}

/// argocd-server on each hub, found once per run.
static REACHED: std::sync::Mutex<BTreeMap<String, Proxied>> =
    std::sync::Mutex::new(BTreeMap::new());

/// argocd-server on the hub, from what was found before or found now.
async fn reach(client: &Client) -> Result<Proxied, String> {
    let held = REACHED
        .lock()
        .ok()
        .and_then(|all| all.get(client.context()).cloned());
    if let Some(to) = held {
        return Ok(to);
    }
    let to = found(client).await?;
    if let Ok(mut all) = REACHED.lock() {
        all.insert(client.context().to_string(), to.clone());
    }
    Ok(to)
}

/// argocd-server's Service on the hub, on plain HTTP when Argo CD serves no TLS itself.
async fn found(client: &Client) -> Result<Proxied, String> {
    let found = client.service(groove_argocd::SERVER_LABEL).await;
    let found = found.map_err(|e| e.to_string())?;
    let found = found.ok_or("no argocd-server Service on the hub")?;
    let insecure = client
        .config_value(&found.namespace, "argocd-cmd-params-cm", "server.insecure")
        .await
        .map_err(|e| e.to_string())?;
    let (scheme, named, default) = match insecure.as_deref() == Some("true") {
        true => ("http", "http", 80),
        false => ("https", "https", 443),
    };
    let port = found.ports.iter().find(|(name, _)| name == named);
    Ok(Proxied {
        namespace: found.namespace.clone(),
        name: found.name.clone(),
        scheme,
        port: port.map_or(default, |(_, port)| *port),
    })
}
