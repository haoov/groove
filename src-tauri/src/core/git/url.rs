//! Parsing git remote URLs into (host, group_path, project).

/// SSH (`git@host:group/project`) and HTTP(S) forms, `.git` stripped, and any
/// userinfo (`user:token@`) removed so a credentialed remote never leaks into
/// the repo id.
pub fn parse_git_url(url: &str) -> anyhow::Result<(String, String, String)> {
    let url = url.trim_end_matches(".git");

    // SSH: git@host:group/project
    if let Some(at) = url.find('@') {
        let rest = &url[at + 1..];
        if let Some(colon) = rest.find(':') {
            let host = &rest[..colon];
            let path = &rest[colon + 1..];
            if let Some(slash) = path.rfind('/') {
                return Ok((
                    host.to_string(),
                    path[..slash].to_string(),
                    path[slash + 1..].to_string(),
                ));
            }
        }
    }

    // HTTPS: https://host/group/project — userinfo (user:token@) stripped, so a
    // credentialed remote never leaks into the repo id.
    let stripped = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let stripped = stripped.rsplit_once('@').map_or(stripped, |(_, rest)| rest);
    if let Some(slash) = stripped.find('/') {
        let host = &stripped[..slash];
        let path = &stripped[slash + 1..];
        if let Some(last_slash) = path.rfind('/') {
            return Ok((
                host.to_string(),
                path[..last_slash].to_string(),
                path[last_slash + 1..].to_string(),
            ));
        }
    }

    Err(anyhow::anyhow!("Cannot parse git URL: {url}"))
}

/// The host of a web URL (`https://gitlab.example.com/g/p/-/merge_requests/4`
/// → `gitlab.example.com`). Userinfo is stripped like everywhere else.
pub fn url_host(url: &str) -> Option<String> {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let rest = rest.rsplit_once('@').map_or(rest, |(_, rest)| rest);
    let host = rest.split('/').next()?.trim();
    (!host.is_empty()).then(|| host.to_string())
}


#[cfg(test)]
mod tests {
    use super::parse_git_url;

    #[test]
    fn parses_ssh_and_https_remotes() {
        assert_eq!(
            parse_git_url("git@gitlab.example.com:group/sub/proj.git").unwrap(),
            ("gitlab.example.com".into(), "group/sub".into(), "proj".into())
        );
        assert_eq!(
            parse_git_url("https://github.com/owner/proj").unwrap(),
            ("github.com".into(), "owner".into(), "proj".into())
        );
    }

    /// A credentialed remote must never leak the token into the repo id.
    #[test]
    fn strips_userinfo_from_https_remotes() {
        let (host, group, project) =
            parse_git_url("https://oauth2:SECRET@gitlab.example.com/group/proj.git").unwrap();
        assert_eq!(host, "gitlab.example.com");
        assert_eq!(group, "group");
        assert_eq!(project, "proj");
    }
}
