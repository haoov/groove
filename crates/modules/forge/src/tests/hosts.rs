use groove_types::{Forge, Repo, RepoId};

use crate::Remote;

fn repo(host: &str) -> Repo {
    Repo {
        id: RepoId::new("r1"),
        host: host.to_string(),
        group_path: "wiremind/devops".into(),
        project: "groove".into(),
        local_path: "/main/groove".into(),
    }
}

#[test]
fn a_host_names_the_forge_that_serves_it() {
    assert_eq!(Forge::of_host("github.com"), Forge::Github);
    assert_eq!(Forge::of_host("github.acme.dev"), Forge::Github);
    assert_eq!(Forge::of_host("gitlab.wiremind.io"), Forge::Gitlab);
    assert_eq!(Forge::of_host("git.internal"), Forge::Gitlab);
}

#[test]
fn a_github_repo_gets_a_client_and_a_gitlab_one_says_so() {
    let remote = Remote::of(&repo("github.com")).expect("a client");
    assert_eq!(remote.kind(), Forge::Github);
    let refused = Remote::of(&repo("gitlab.wiremind.io"))
        .err()
        .expect("no client yet");
    assert!(refused.to_string().contains("GitLab"), "{refused}");
}
