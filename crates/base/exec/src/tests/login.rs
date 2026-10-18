use crate::login::*;

#[test]
fn marked_takes_the_path_between_the_marks() {
    let out = "welcome banner\n__GROOVE_PATH__/usr/bin:/opt/bin__GROOVE_PATH__";
    assert_eq!(marked(out).as_deref(), Some("/usr/bin:/opt/bin"));
}

#[test]
fn marked_is_none_without_both_marks_or_a_path() {
    assert_eq!(marked("/usr/bin"), None);
    assert_eq!(marked("__GROOVE_PATH__/usr/bin"), None);
    assert_eq!(marked("__GROOVE_PATH____GROOVE_PATH__"), None);
}

#[tokio::test]
async fn path_reads_what_the_shell_exports() {
    let path = path("sh").await.unwrap();
    assert!(
        path.split(':')
            .any(|dir| dir == "/usr/bin" || dir == "/bin")
    );
}

#[tokio::test]
async fn path_falls_back_to_the_non_interactive_shell() {
    let dir = std::env::temp_dir().join(format!("groove-login-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let shell = dir.join("shell");
    let script = "#!/bin/sh\ncase \"$2\" in -i) exit 1;; esac\nPATH=/plain/bin\neval \"$3\"\n";
    std::fs::write(&shell, script).unwrap();
    std::process::Command::new("chmod")
        .args(["+x", shell.to_str().unwrap()])
        .status()
        .unwrap();
    let path = path(shell.to_str().unwrap()).await;
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(path.as_deref(), Some("/plain/bin"));
}

#[test]
fn joined_adds_only_the_missing_dirs_at_the_end() {
    let extra = ["/opt/homebrew/bin", "/usr/local/bin"];
    assert_eq!(
        joined("/usr/bin:/usr/local/bin", &extra),
        "/usr/bin:/usr/local/bin:/opt/homebrew/bin"
    );
    assert_eq!(joined("", &extra), "/opt/homebrew/bin:/usr/local/bin");
}

#[tokio::test]
async fn path_is_none_when_the_shell_is_missing() {
    assert_eq!(path("/nonexistent/shell").await, None);
}
