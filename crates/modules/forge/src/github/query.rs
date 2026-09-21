//! The GraphQL Groove sends: one field set, and the two ways to ask for it.

/// Everything one read of an MR brings back, down to the threads.
const FIELDS: &str = r"
  id number title body state isDraft url createdAt updatedAt
  headRefName baseRefName reviewDecision
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
      id isResolved path line diffSide
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
  viewer {{ login }}
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
  viewer {{ login }}
  updatePullRequest(input: {{ pullRequestId: $mr, title: $title, body: $body }})
    {{ pullRequest {{ {FIELDS} }} }}
}}"
    )
}

/// The merge request closed, with nothing merged.
pub fn shut() -> String {
    format!(
        r"mutation($mr: ID!) {{
  viewer {{ login }}
  closePullRequest(input: {{ pullRequestId: $mr }}) {{ pullRequest {{ {FIELDS} }} }}
}}"
    )
}
