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
async fn path_is_none_when_the_shell_is_missing() {
    assert_eq!(path("/nonexistent/shell").await, None);
}
