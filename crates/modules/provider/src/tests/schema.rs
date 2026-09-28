use groove_types::Kind;
use serde_json::json;

use crate::github::schema::fields;
use crate::notion::schema::properties;

#[test]
fn a_notion_database_gives_each_property_its_type_and_its_values() {
    let database = json!({ "properties": {
        "Status": { "type": "status", "status": { "options": [{ "name": "Todo" }, { "name": "Done" }] } },
        "Due date": { "type": "date", "date": {} },
        "Assignee": { "type": "people", "people": {} },
        "Task name": { "type": "title", "title": {} }
    } });
    let found = properties(&database);
    let names: Vec<_> = found
        .iter()
        .map(|one| (one.name.as_str(), one.kind))
        .collect();
    assert_eq!(
        names,
        [
            ("Assignee", Kind::People),
            ("Due date", Kind::Date),
            ("Status", Kind::Status),
            ("Task name", Kind::Other)
        ]
    );
    assert_eq!(found[2].options, ["Todo", "Done"]);
}

#[test]
fn github_boards_give_each_field_once_with_every_option_they_hold() {
    let board = |options: &[&str]| {
        let options: Vec<_> = options.iter().map(|name| json!({ "name": name })).collect();
        json!({ "project": { "fields": { "nodes": [
            { "name": "Status", "dataType": "SINGLE_SELECT", "options": options },
            { "name": "Target date", "dataType": "DATE" }
        ] } } })
    };
    let reply = json!({ "data": { "search": { "nodes": [
        { "projectItems": { "nodes": [board(&["Todo", "Done"])] } },
        { "projectItems": { "nodes": [board(&["Todo", "In review"])] } }
    ] } } });
    let found = fields(&reply);
    assert_eq!(found.len(), 2, "one per name, whatever the board");
    assert_eq!(
        (found[0].name.as_str(), found[0].kind),
        ("Status", Kind::Select)
    );
    assert_eq!(found[0].options, ["Todo", "Done", "In review"]);
    assert_eq!(found[1].kind, Kind::Date);
}
