//! The GraphQL Groove sends: one field set, and the two ways to ask for it.

/// Everything one read of an MR brings back, down to the threads.
const FIELDS: &str = r"
  number title body state isDraft url createdAt updatedAt
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
