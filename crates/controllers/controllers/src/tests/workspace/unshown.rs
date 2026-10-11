//! A file that is no text opens as a stand-in, which no edit and no save write back.

use super::*;

#[test]
fn a_binary_file_opened_takes_no_edit_and_is_never_saved_over() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    pooled_clone(home.path());
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let file = editing(&mut state, &services, &spawner);
    let bytes = b"\x89PNG\r\n\x1a\n\0\0\0".to_vec();
    std::fs::write(&file, &bytes).unwrap();
    act(
        &mut state,
        &services,
        &spawner,
        workspace::Command::CloseFile {
            path: "a.txt".into(),
        },
    );
    act(
        &mut state,
        &services,
        &spawner,
        workspace::Command::OpenFile {
            at: None,
            path: "a.txt".into(),
        },
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.active().is_some()
    });
    assert_eq!(buffer(&state), groove_workspace_service::UNSHOWN);
    edit(&mut state, &services, &spawner, &[Edit::Insert("x".into())]);
    assert_eq!(buffer(&state), groove_workspace_service::UNSHOWN);
    act(
        &mut state,
        &services,
        &spawner,
        workspace::Command::SaveFile,
    );
    until(&spawner, &services, &mut state, |s| s.pending.is_empty());
    assert_eq!(
        std::fs::read(&file).unwrap(),
        bytes,
        "the disk keeps its bytes"
    );
}
