//! The GraphQL Groove sends GitLab: one field set, and the calls that carry it.

/// Everything one read of an MR brings back, down to the discussions.
const FIELDS: &str = r"
  id iid title description state draft webUrl createdAt updatedAt
  sourceBranch targetBranch
  author { username }
  approved
  approvedBy { nodes { username } }
  reviewers { nodes { username mergeRequestInteraction { reviewState } } }
  headPipeline { status finishedAt path }
  discussions(first: 50) {
    nodes {
      id resolved
      notes {
        nodes {
          body createdAt resolvable resolved
          author { username }
          position { newPath newLine oldPath oldLine }
        }
      }
    }
  }
";

/// The open MR this branch is the source of, the one most lately touched.
pub fn by_branch() -> String {
    format!(
        r"query($path: ID!, $branch: String!) {{
  currentUser {{ username }}
  project(fullPath: $path) {{
    mergeRequests(
      sourceBranches: [$branch], state: opened, first: 1, sort: UPDATED_DESC
    ) {{ nodes {{ {FIELDS} }} }}
  }}
}}"
    )
}

/// One MR, by the number its row remembers.
pub fn by_iid() -> String {
    format!(
        r"query($path: ID!, $iid: String!) {{
  currentUser {{ username }}
  project(fullPath: $path) {{ mergeRequest(iid: $iid) {{ {FIELDS} }} }}
}}"
    )
}

/// The branch a project merges into by default.
pub fn root_ref() -> String {
    r"query($path: ID!) {
  project(fullPath: $path) { repository { rootRef } }
}"
    .to_string()
}

/// One MR opened, and read back in the same call.
pub fn open() -> String {
    format!(
        r"mutation($path: ID!, $head: String!, $base: String!, $title: String!, $body: String!) {{
  currentUser {{ username }}
  mergeRequestCreate(input: {{
    projectPath: $path, sourceBranch: $head, targetBranch: $base,
    title: $title, description: $body
  }}) {{ errors mergeRequest {{ {FIELDS} }} }}
}}"
    )
}

/// Its title and its body written again.
pub fn edit() -> String {
    format!(
        r"mutation($path: ID!, $iid: String!, $title: String!, $body: String!) {{
  currentUser {{ username }}
  mergeRequestUpdate(input: {{
    projectPath: $path, iid: $iid, title: $title, description: $body
  }}) {{ errors mergeRequest {{ {FIELDS} }} }}
}}"
    )
}

/// The MR closed, with nothing merged.
pub fn shut() -> String {
    format!(
        r"mutation($path: ID!, $iid: String!) {{
  currentUser {{ username }}
  mergeRequestUpdate(input: {{ projectPath: $path, iid: $iid, state: CLOSED }})
    {{ errors mergeRequest {{ {FIELDS} }} }}
}}"
    )
}

/// Every open MR the viewer is asked to review.
pub fn review_queue() -> String {
    r"query($first: Int!) {
  currentUser {
    reviewRequestedMergeRequests(state: opened, first: $first, sort: UPDATED_DESC) {
      nodes {
        iid title webUrl draft updatedAt sourceBranch targetBranch approved
        author { username }
        project { fullPath }
      }
    }
  }
}"
    .to_string()
}
