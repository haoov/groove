//! The GraphQL Groove sends GitLab: one field set, and the calls that carry it.

/// Everything one read of an MR brings back, down to the discussions.
const FIELDS: &str = r"
  id iid title description state draft webUrl createdAt updatedAt
  diffHeadSha sourceBranch targetBranch
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

/// Who the token is; GitLab answers `currentUser` in a query, never beside a mutation.
pub fn viewer() -> String {
    "query { currentUser { username } }".to_string()
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
  mergeRequestCreate(input: {{
    projectPath: $path, sourceBranch: $head, targetBranch: $base,
    title: $title, description: $body
  }}) {{ errors mergeRequest {{ {FIELDS} }} }}
}}"
    )
}

/// The viewer added to the MR's assignees.
pub fn assign() -> String {
    r"mutation($path: ID!, $iid: String!, $who: [String!]!) {
  mergeRequestSetAssignees(input: {
    projectPath: $path, iid: $iid, assigneeUsernames: $who, operationMode: APPEND
  }) { errors }
}"
    .to_string()
}

/// Its title and its body written again.
pub fn edit() -> String {
    format!(
        r"mutation($path: ID!, $iid: String!, $title: String!, $body: String!) {{
  mergeRequestUpdate(input: {{
    projectPath: $path, iid: $iid, title: $title, description: $body
  }}) {{ errors mergeRequest {{ {FIELDS} }} }}
}}"
    )
}

/// The MR closed, with nothing merged.
pub fn close() -> String {
    format!(
        r"mutation($path: ID!, $iid: String!) {{
  mergeRequestUpdate(input: {{ projectPath: $path, iid: $iid, state: CLOSED }})
    {{ errors mergeRequest {{ {FIELDS} }} }}
}}"
    )
}

/// Every open MR the viewer is asked to review.
pub fn review_queue() -> String {
    r"query($first: Int!) {
  currentUser {
    username
    reviewRequestedMergeRequests(state: opened, first: $first, sort: UPDATED_DESC) {
      nodes {
        iid title webUrl draft updatedAt sourceBranch targetBranch approved
        author { username }
        reviewers { nodes { username mergeRequestInteraction { reviewState } } }
        project { fullPath }
      }
    }
  }
}"
    .to_string()
}

/// A note on the latest diff. Only a note over more than one line names its last.
pub fn note_on_line(range: bool) -> String {
    let (takes, ends) = match range {
        true => (", $to: Int!", "endNewLine: $to, "),
        false => ("", ""),
    };
    format!(
        "mutation($mr: MergeRequestID!, $head: String!, $path: String!, $from: Int!{takes}, \
         $body: String!) {{
  createLatestDiffNote(input: {{
    noteableId: $mr, headSha: $head, filePath: $path,
    newLine: $from, {ends}body: $body
  }}) {{ errors note {{ id }} }}
}}"
    )
}

/// A reply under a discussion that stands.
pub fn reply() -> String {
    r"mutation($mr: NoteableID!, $thread: DiscussionID!, $body: String!) {
  createNote(input: { noteableId: $mr, discussionId: $thread, body: $body })
    { errors note { id } }
}"
    .to_string()
}

/// A discussion resolved, or opened again.
pub fn resolve() -> String {
    r"mutation($thread: DiscussionID!, $resolve: Boolean!) {
  discussionToggleResolve(input: { id: $thread, resolve: $resolve })
    { errors discussion { resolved } }
}"
    .to_string()
}

/// A comment on the merge request itself, under no discussion.
pub fn comment() -> String {
    r"mutation($mr: NoteableID!, $body: String!) {
  createNote(input: { noteableId: $mr, body: $body }) { errors note { id } }
}"
    .to_string()
}

/// The changes a reviewer asks for, which GitLab addresses by path and number.
pub fn request_changes() -> String {
    r"mutation($path: ID!, $iid: String!) {
  mergeRequestRequestChanges(input: { projectPath: $path, iid: $iid }) { errors }
}"
    .to_string()
}
