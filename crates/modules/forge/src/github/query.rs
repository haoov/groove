//! The GraphQL Groove sends: one field set, and the two ways to ask for it.

/// Everything one read of an MR brings back, down to the threads.
const FIELDS: &str = r"
  id number title body state isDraft url createdAt updatedAt
  headRefName headRefOid baseRefName reviewDecision
  author { login }
  latestReviews(first: 50) { nodes { state submittedAt author { login } } }
  reviewRequests(first: 50) { nodes { requestedReviewer { ... on User { login } } } }
  timelineItems(itemTypes: [REVIEW_REQUESTED_EVENT], last: 20) {
    nodes {
      ... on ReviewRequestedEvent {
        createdAt
        requestedReviewer { ... on User { login } }
      }
    }
  }
  commits(last: 1) {
    nodes {
      commit {
        statusCheckRollup {
          contexts(first: 100) {
            nodes {
              __typename
              ... on CheckRun { status conclusion detailsUrl completedAt }
              ... on StatusContext { state targetUrl createdAt }
            }
          }
        }
      }
    }
  }
  reviewThreads(first: 50) {
    nodes {
      id isResolved path line startLine originalStartLine diffSide
      comments(first: 50) {
        nodes { body createdAt line originalLine author { login } }
      }
    }
  }
  comments(first: 50) { nodes { id body createdAt author { login } } }
";

/// The open MR a branch is the source of, the one most lately touched.
pub fn by_branch() -> String {
    format!(
        r"query($owner: String!, $repo: String!, $branch: String!) {{
  viewer {{ login }}
  repository(owner: $owner, name: $repo) {{
    pullRequests(
      headRefName: $branch, states: [OPEN], first: 1
      orderBy: {{ field: UPDATED_AT, direction: DESC }}
    ) {{ nodes {{ {FIELDS} }} }}
  }}
}}"
    )
}

/// One MR, by the number its row remembers.
pub fn by_number() -> String {
    format!(
        r"query($owner: String!, $repo: String!, $number: Int!) {{
  viewer {{ login }}
  repository(owner: $owner, name: $repo) {{ pullRequest(number: $number) {{ {FIELDS} }} }}
}}"
    )
}

/// The repository a merge request is opened on.
pub fn repository() -> String {
    r"query($owner: String!, $repo: String!) {
  repository(owner: $owner, name: $repo) { id defaultBranchRef { name } }
}"
    .to_string()
}

/// One merge request opened, and read back in the same call.
pub fn open() -> String {
    format!(
        r"mutation($repo: ID!, $base: String!, $head: String!, $title: String!, $body: String!) {{
  createPullRequest(input: {{
    repositoryId: $repo, baseRefName: $base, headRefName: $head, title: $title, body: $body
  }}) {{ pullRequest {{ {FIELDS} }} }}
}}"
    )
}

/// Its title and its body written again.
pub fn edit() -> String {
    format!(
        r"mutation($mr: ID!, $title: String!, $body: String!) {{
  updatePullRequest(input: {{ pullRequestId: $mr, title: $title, body: $body }})
    {{ pullRequest {{ {FIELDS} }} }}
}}"
    )
}

/// The merge request closed, with nothing merged.
pub fn close() -> String {
    format!(
        r"mutation($mr: ID!) {{
  closePullRequest(input: {{ pullRequestId: $mr }}) {{ pullRequest {{ {FIELDS} }} }}
}}"
    )
}

/// Who the token is; GitHub answers `viewer` in a query, never beside a mutation.
pub fn viewer() -> String {
    "query { viewer { id login } }".to_string()
}

/// The viewer added to the MR's assignees.
pub fn assign() -> String {
    r"mutation($mr: ID!, $who: [ID!]!) {
  addAssigneesToAssignable(input: { assignableId: $mr, assigneeIds: $who }) { clientMutationId }
}"
    .to_string()
}

/// Every open merge request the viewer is asked to review.
pub fn review_queue() -> String {
    r#"query($first: Int!) {
  viewer { login }
  search(query: "is:open is:pr review-requested:@me archived:false", type: ISSUE, first: $first) {
    nodes {
      ... on PullRequest {
        number title url isDraft updatedAt reviewDecision
        author { login }
        latestReviews(first: 20) { nodes { state submittedAt author { login } } }
        reviewRequests(first: 20) { nodes { requestedReviewer { ... on User { login } } } }
        headRefName baseRefName
        repository { nameWithOwner }
      }
    }
  }
}"#
    .to_string()
}

/// A thread on one file's new side. Only a range names the line it starts on.
pub fn note_on_line(range: bool) -> String {
    let (takes, starts) = match range {
        true => (", $from: Int!", "startLine: $from, startSide: RIGHT, "),
        false => ("", ""),
    };
    format!(
        "mutation($mr: ID!, $path: String!, $to: Int!{takes}, $body: String!) {{
  addPullRequestReviewThread(input: {{
    pullRequestId: $mr, path: $path, line: $to, {starts}side: RIGHT, body: $body
  }}) {{ thread {{ id }} }}
}}"
    )
}

/// A reply under a thread that stands.
pub fn reply() -> String {
    r"mutation($thread: ID!, $body: String!) {
  addPullRequestReviewThreadReply(input: {
    pullRequestReviewThreadId: $thread, body: $body
  }) { comment { id } }
}"
    .to_string()
}

/// A thread resolved, or opened again.
pub fn resolve() -> String {
    r"mutation($thread: ID!) {
  resolveReviewThread(input: { threadId: $thread }) { thread { id } }
}"
    .to_string()
}

pub fn unresolve() -> String {
    r"mutation($thread: ID!) {
  unresolveReviewThread(input: { threadId: $thread }) { thread { id } }
}"
    .to_string()
}

/// A review with its verdict, its words, and the threads it opens.
pub fn review() -> String {
    r"mutation($mr: ID!, $event: PullRequestReviewEvent!, $body: String!, $threads: [DraftPullRequestReviewThread!]) {
  addPullRequestReview(input: {
    pullRequestId: $mr, event: $event, body: $body, threads: $threads
  }) { pullRequestReview { id } }
}"
    .to_string()
}

/// A comment on the merge request itself.
pub fn comment() -> String {
    r"mutation($mr: ID!, $body: String!) {
  addComment(input: { subjectId: $mr, body: $body }) { clientMutationId }
}"
    .to_string()
}
