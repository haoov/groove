use crate::{windows, windows_path};

#[test]
fn the_windows_read_with_which_are_active_and_whether_it_may_sync() {
    let deny = r#"{"kind":"deny","schedule":"0 22 * * *","duration":"8h",
        "timeZone":"Europe/Paris","manualSync":true}"#;
    let body =
        format!(r#"{{"assignedWindows":[{deny}],"activeWindows":[{deny}],"canSync":false}}"#);
    let read = windows(&body).expect("windows");
    assert!(!read.can_sync);
    assert_eq!(read.assigned, read.active);
    let one = &read.assigned[0];
    assert_eq!((one.kind.as_str(), one.duration.as_str()), ("deny", "8h"));
    assert!(one.manual);
}

#[test]
fn no_window_reads_as_free_to_sync_and_a_refusal_as_what_argo_cd_said() {
    let read = windows(r#"{"canSync":true}"#).expect("windows");
    assert!(read.can_sync && read.assigned.is_empty());
    let refused = windows(r#"{"code":7,"message":"permission denied"}"#);
    assert_eq!(refused, Err("permission denied".to_string()));
    let path = windows_path("argocd", "shop");
    assert_eq!(
        path,
        "/api/v1/applications/shop/syncwindows?appNamespace=argocd"
    );
}
