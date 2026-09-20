//! The GraphQL Groove sends: the two reads, and the one write it makes itself.

/// What both queries read from an issue. The ids are what a write needs.
const FIELDS: &str = r"
  number title url body
  repository { name owner { login } }
  projectItems(first: 5) {
    nodes {
      id
      project {
        id title
        fields(first: 50) {
          nodes {
            ... on ProjectV2FieldCommon { id name }
            ... on ProjectV2SingleSelectField { options { id name } }
          }
        }
      }
      fieldValues(first: 25) {
        nodes {
          __typename
          ... on ProjectV2ItemFieldSingleSelectValue {
            name field { ... on ProjectV2SingleSelectField { name } }
          }
          ... on ProjectV2ItemFieldTextValue {
            text field { ... on ProjectV2Field { name } }
          }
          ... on ProjectV2ItemFieldNumberValue {
            number field { ... on ProjectV2Field { name } }
          }
          ... on ProjectV2ItemFieldDateValue {
            date field { ... on ProjectV2Field { name } }
          }
        }
      }
    }
  }
";

/// One of a field's own options into one field of one board item.
pub fn set_select() -> String {
    r"mutation($project: ID!, $item: ID!, $field: ID!, $option: String!) {
  updateProjectV2ItemFieldValue(input: {
    projectId: $project, itemId: $item, fieldId: $field, value: { singleSelectOptionId: $option }
  }) { projectV2Item { id } }
}"
    .to_string()
}

/// One number into one field of one board item.
pub fn set_number() -> String {
    r"mutation($project: ID!, $item: ID!, $field: ID!, $value: Float!) {
  updateProjectV2ItemFieldValue(input: {
    projectId: $project, itemId: $item, fieldId: $field, value: { number: $value }
  }) { projectV2Item { id } }
}"
    .to_string()
}

/// Every open issue assigned to the viewer.
pub fn assigned() -> String {
    format!(
        r#"query($after: String) {{
  search(query: "assignee:@me is:issue is:open", type: ISSUE, first: 50, after: $after) {{
    pageInfo {{ hasNextPage endCursor }}
    nodes {{ ... on Issue {{ {FIELDS} }} }}
  }}
}}"#
    )
}

/// One issue, by owner, repo and number.
pub fn issue() -> String {
    format!(
        r"query($owner: String!, $repo: String!, $number: Int!) {{
  repository(owner: $owner, name: $repo) {{ issue(number: $number) {{ {FIELDS} }} }}
}}"
    )
}
