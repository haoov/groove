use crate::{Tool, all, named, wording};

fn schema_of(tool: &Tool) -> (&serde_json::Value, &serde_json::Value) {
    (&tool.schema["type"], &tool.schema["properties"])
}

#[test]
fn no_two_tools_share_a_name() {
    let mut names: Vec<&str> = all().iter().map(|tool| tool.name).collect();
    names.sort_unstable();
    let held = names.len();
    names.dedup();
    assert_eq!(names.len(), held, "a name answers for one tool");
}

#[test]
fn every_tool_takes_an_object_and_says_what_it_is_for() {
    for tool in all() {
        let (kind, properties) = schema_of(&tool);
        assert_eq!(kind, "object", "{}", tool.name);
        assert!(properties.is_object(), "{}", tool.name);
        assert!(tool.description.len() > 20, "{} says too little", tool.name);
    }
}

#[test]
fn every_required_argument_is_one_the_tool_takes() {
    for tool in all() {
        let required = tool.schema["required"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for one in required {
            let name = one.as_str().unwrap_or_default();
            assert!(
                !tool.schema["properties"][name].is_null(),
                "{} asks for {name} and does not take it",
                tool.name
            );
        }
    }
}

#[test]
fn every_argument_says_what_it_is_for() {
    for tool in all() {
        let Some(properties) = tool.schema["properties"].as_object() else {
            continue;
        };
        for (name, one) in properties {
            assert!(
                one["description"].is_string(),
                "{}'s {name} says nothing",
                tool.name
            );
        }
    }
}

#[test]
fn a_tool_that_changes_something_says_so() {
    let writes: Vec<&str> = all()
        .iter()
        .filter(|tool| tool.writes)
        .map(|tool| tool.name)
        .collect();
    for named in [
        "git_commit",
        "create_mr",
        "create_annotation",
        "finish_task",
    ] {
        assert!(writes.contains(&named), "{named}");
    }
    let reads: Vec<&str> = all()
        .iter()
        .filter(|tool| !tool.writes)
        .map(|tool| tool.name)
        .collect();
    for named in ["get_active_task", "get_mr_state", "read_file"] {
        assert!(reads.contains(&named), "{named}");
    }
}

#[test]
fn the_rules_for_what_a_human_reads_are_said_at_the_call() {
    let commit = named("git_commit").expect("the commit tool");
    let message = commit.schema["properties"]["message"]["description"]
        .as_str()
        .expect("what a message says");
    assert!(message.contains("Conventional commit subject"), "{message}");

    let mr = named("create_mr").expect("the mr tool");
    let body = mr.schema["properties"]["description"]["description"]
        .as_str()
        .expect("what a body says");
    assert!(body.contains("## What"), "{body}");
    assert!(body.contains(wording::TIGHT), "{body}");

    let note = named("create_annotation").expect("the note tool");
    let said = note.schema["properties"]["content"]["description"]
        .as_str()
        .expect("what a note says");
    assert!(said.contains("Conventional Comment"), "{said}");
}

#[test]
fn a_tool_is_listed_as_the_harness_reads_it() {
    let one = named("get_mr_state").expect("the state tool");
    let listed = one.listed();
    assert_eq!(listed["name"], "get_mr_state");
    assert!(listed["description"].is_string());
    assert_eq!(listed["inputSchema"]["type"], "object");
}

#[test]
fn a_name_nothing_answers_for_is_none() {
    assert!(named("rm_rf").is_none());
}
