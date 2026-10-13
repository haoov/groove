//! What the agent reads of the session's clusters, and the clusters it attaches or lets go.

use crate::{Tool, strings, takes, text};

/// The read that runs kubectl for the agent; it changes nothing.
fn kube() -> Tool {
    Tool {
        name: "kube",
        description: "Run a read-only kubectl command on a cluster this session holds. You \
             never run kubectl yourself: Groove checks the command and runs it. Reads only \
             (get, describe, logs, events, top, explain, api-resources, api-versions, \
             version, auth can-i, rollout history, rollout status). The namespace must be \
             one the session holds. A secret's data is never shown. The answer is cut at \
             32 KiB; narrow it with a selector, -o jsonpath, --tail or --limit-bytes. A \
             secret shows its metadata only, by named field: -o jsonpath={.metadata.managedFields}."
            .to_string(),
        schema: takes(
            &["context", "args"],
            vec![
                (
                    "context",
                    text("The kubeconfig context, one the session holds."),
                ),
                (
                    "args",
                    strings(
                        "The kubectl arguments, one per item, without `kubectl` and \
                         without --context: [\"get\", \"pods\", \"-n\", \"api\", \"-o\", \"wide\"].",
                    ),
                ),
                (
                    "jq",
                    text("A jq filter applied to the JSON answer; ask for -o json with it."),
                ),
            ],
        ),
        writes: false,
        verb: "read",
        subject: &["context"],
    }
}

/// A change to the clusters the session holds, which the user decides on.
fn attaching(name: &'static str, verb: &'static str, description: &str) -> Tool {
    Tool {
        name,
        description: description.to_string(),
        schema: takes(
            &["context"],
            vec![
                (
                    "context",
                    text("A context added in Groove's Settings › Clusters."),
                ),
                (
                    "namespace",
                    text("One namespace; leave it out for the whole cluster."),
                ),
            ],
        ),
        writes: true,
        verb,
        subject: &["context"],
    }
}

pub(crate) fn all() -> Vec<Tool> {
    vec![
        kube(),
        attaching(
            "attach_cluster",
            "attach a cluster",
            "Attach a cluster context to this session, on one namespace or the whole \
             cluster, so kube can read it.",
        ),
        attaching(
            "detach_cluster",
            "detach a cluster",
            "Let go of a cluster context or one of its namespaces this session holds.",
        ),
    ]
}
