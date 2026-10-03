//! The agent's MR writes, through the controller to a forge that answers or refuses.

use groove_delivery_service::{Github, Remote, Token};
use serde_json::json;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::asked;
use crate::tests::fixture::{self, worktree};
use crate::{AppState, Services, SyncSpawner};

fn pr() -> serde_json::Value {
    json!({
        "number": 7, "id": "PR_node", "title": "fix: one", "body": "why", "state": "OPEN",
        "isDraft": false, "url": "https://forge.test/acme/mayo/pull/7",
        "createdAt": "2026-09-18T08:00:00Z", "updatedAt": "2026-09-19T09:30:00Z",
        "headRefName": "explorer/x", "headRefOid": "cafe1234", "baseRefName": "main",
        "author": { "login": "haoov" }
    })
}

/// A session on a worktree whose forge is the mock, its writes run without asking.
struct OnForge {
    _runtime: tokio::runtime::Runtime,
    _server: MockServer,
    _home: tempfile::TempDir,
    spawner: SyncSpawner,
    services: Services,
    state: AppState,
    session: String,
}

impl OnForge {
    fn new(answer: ResponseTemplate) -> Self {
        let runtime = tokio::runtime::Runtime::new().expect("a runtime");
        let server = runtime.block_on(async {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(answer)
                .mount(&server)
                .await;
            server
        });
        let (home, spawner, mut services, mut state) = fixture::fresh();
        fixture::pooled_clone(home.path());
        worktree(&mut state, &services, &spawner);
        let host = format!("http://{}", server.address());
        services.delivery = services.delivery.clone().connecting(move |_| {
            Ok(Remote::Github(Github::with_token(
                &host,
                Token::Fixed("t".into()),
            )?))
        });
        let session = state.session.selected.clone().expect("a session");
        crate::tests::fixture::auto_approve(&mut state, &session);
        let session = session.to_string();
        Self {
            _runtime: runtime,
            _server: server,
            _home: home,
            spawner,
            services,
            state,
            session,
        }
    }

    fn opens(&mut self) -> groove_agent_service::Answer {
        let args = json!({ "title": "fix: one", "description": "why" });
        let (state, session) = (&mut self.state, &self.session);
        asked(
            state,
            &self.services,
            &self.spawner,
            session,
            "create_mr",
            args,
        )
    }

    fn held(&self) -> Option<&groove_delivery_service::Held> {
        let worktree = self.state.session.selected_worktree().expect("a worktree");
        self.state.delivery.held(&worktree.id)
    }
}

#[test]
fn an_mr_the_agent_opens_lands_on_its_worktree() {
    let reply = json!({ "data": {
        "viewer": { "id": "U_me", "login": "haoov" },
        "repository": { "id": "R_1", "defaultBranchRef": { "name": "main" } },
        "createPullRequest": { "pullRequest": pr() },
        "addAssigneesToAssignable": { "clientMutationId": null }
    }});
    let mut forge = OnForge::new(ResponseTemplate::new(200).set_body_json(reply));
    let answer = forge.opens();
    assert!(!answer.failed, "{}", answer.text);
    let held = forge.held().expect("the MR is on its row");
    assert_eq!(held.shown().map(|mr| mr.number), Some("7".to_string()));
}

#[test]
fn a_forge_that_refuses_the_mr_is_the_agent_s_answer() {
    let mut forge = OnForge::new(ResponseTemplate::new(502));
    let answer = forge.opens();
    assert!(answer.failed, "the agent hears it failed: {}", answer.text);
    assert!(forge.held().is_none(), "no MR on the row");
}
