use groove_types::FileStatus;

use crate::{from_pairs, unified};

fn pair(path: &str, old: &str, new: &str) -> (String, String, String) {
    (path.into(), old.into(), new.into())
}

#[test]
fn pairs_read_as_one_stream_each_file_counted_and_an_empty_side_added_or_deleted() {
    let (files, changes) = from_pairs(vec![
        pair(
            "argo/Deployment/shop/web.yaml",
            "replicas: 2\n",
            "replicas: 3\n",
        ),
        pair("argo/ConfigMap/shop/old.yaml", "a: 1\n", ""),
        pair("argo/Service/shop/new.yaml", "", "b: 2\n"),
    ]);
    let statuses: Vec<FileStatus> = files.iter().map(|one| one.status).collect();
    assert_eq!(
        statuses,
        [FileStatus::Modified, FileStatus::Deleted, FileStatus::Added]
    );
    assert_eq!((files[0].added, files[0].deleted), (1, 1));
    assert!(!changes.is_empty());
}

#[test]
fn a_change_reads_as_text_with_its_signs() {
    let text = unified(
        "web.yaml",
        "replicas: 2\nimage: a\n",
        "replicas: 3\nimage: a\n",
    );
    assert!(text.starts_with("--- web.yaml\n"), "{text}");
    assert!(text.contains("- replicas: 2"), "{text}");
    assert!(text.contains("+ replicas: 3"), "{text}");
    assert!(text.contains("  image: a"), "{text}");
}
