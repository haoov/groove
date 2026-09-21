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
fn each_host_gets_the_client_its_own_forge_asks_for() {
    let github = Remote::of(&repo("github.com")).expect("a github client");
    assert_eq!(github.kind(), Forge::Github);
    let gitlab = Remote::of(&repo("gitlab.wiremind.io")).expect("a gitlab client");
    assert_eq!(gitlab.kind(), Forge::Gitlab);
}
