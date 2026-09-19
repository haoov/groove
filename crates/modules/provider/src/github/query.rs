//! The two GraphQL queries: every task of yours, and one issue by number.

/// What both queries read from an issue.
const FIELDS: &str = r"
  number title url body
  repository { name owner { login } }
  projectItems(first: 5) {
    nodes {
      project { title }
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
