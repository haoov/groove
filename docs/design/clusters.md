# Clusters — design

The last step after task, code, MR and delivery: the clusters the work runs on. Designed,
not built. It touches four capabilities — Config holds the contexts, Sessions attaches
them, Workspace shows them, Agent acts on them — and it follows
[architecture.md](architecture.md) like every other feature.

## What it answers

- The cluster is part of the session, like its repos: the user and the agent see the same
  resources, from the same place.
- The agent works on a cluster through a harness, not through a raw `kubectl`: it reads
  what the session holds, and every write it makes waits on the user.
- Every edit of a resource shows its diff before it applies.

## Contexts

**A cluster is added to Groove before a session can use it.** Settings lists the contexts
Groove knows, each one added from the kubeconfig. Groove keeps a reference to the kubeconfig
context; the credentials stay there, `kubelogin` and exec plugins included. A context Groove
does not know cannot be attached, by the user or the agent.

**Each context has a Groove configuration of its own:**

- read-only: no write at all, for the user or the agent;
- a display name and a colour, shown everywhere the context appears;
- the namespaces offered first when it is attached;
- whether it is the Argo CD hub, whose Applications fan out to the other clusters.

**An expired login is said, not hidden.** Groove names the context whose token failed and
offers the sign-in in a terminal, as it does for the agent's own sign-in.

## Attaching

**A session holds cluster and namespace pairs, as it holds repos.** They are attached and
detached the way a repo is added and removed, and they stay with the session across
restarts.

**The overview names what is attached, and nothing else.** A section beside "Repos and
worktrees" lists each cluster and its namespaces. No health, no counts.

**A session with nothing attached has no Resources tab.**

## The Resources tab

A tab of the workspace, beside Overview, Diff and Files. It follows the workspace's layout:
a sidebar, and a main panel with tabs.

**The header's pickers choose the scope, and take more than one.** A cluster picker and a
namespace picker, each a multi-select over what the session holds. When the scope holds more
than one cluster or namespace, the list gains a Cluster column and a Namespace column, and a
resource tab carries both in its title.

**The sidebar lists the kinds.** Grouped — workloads, network, config, storage, Argo, custom
— the custom kinds found through API discovery. Each kind shows its count, and a mark when a
resource of it is not healthy. Cluster-wide kinds — nodes, namespaces, persistent volumes,
storage classes, CRDs, cluster roles — are always listed, whatever namespaces are picked.

**The main panel holds the list tab, which never closes, and a tab per open resource.**

- The list shows the resources of the selected kind in the scope, and changes in place as
  the cluster does. Core kinds have built-in columns; a custom kind takes the columns its
  CRD declares.
- Search is the workspace's search bar: it filters the list by name or label.
- A click on a row opens the resource in a tab of its own: its description — status,
  conditions, events — and its YAML; a pod's tab also holds its logs.

**A shell into a pod is a terminal of the session**, in the bottom section, named after the
pod and the container.

## Writes

Three paths, one rule each:

| Path | What stands before the write |
|---|---|
| An action from the list: restart, scale, delete | A confirmation on the row itself, as discarding a file has |
| An edit of the YAML, saved | The server-side dry run's full diff, in the review sheet; Enter applies it |
| A write by the agent | The approval queue, with the same full diff. Auto-approve never lets it through |

**The sheet shows the target before the diff**: cluster, in its colour, namespace, kind and
name. The diff is always whole.

**A read-only context has no write path at all.** Its rows have no actions, its YAML does not
save, and the agent's write tools refuse it.

**A write that lands is a line of the session's timeline**: the target and what changed. The
user's writes and the agent's alike.

**A resource that moved while it was edited is not overwritten.** The apply is refused, and
the tab says the object changed.

## The object pipeline

The view, the overview and the agent read from one store per cluster. The goal is what
sofka shows: the interface never waits on the cluster, and Groove fetches no more than it
shows.

**One watcher per cluster, kind and namespace set, shared.** A watcher runs while something
reads it — a list on screen, an open resource tab, a recent agent read — and stops a short
time after the last of them. A session costs what its screens show.

**Fetch less, not only draw less:**

- a list reads metadata only when its columns need no more; the whole object is read when
  its tab opens;
- selectors filter on the server, never in Groove;
- the first list is paged, then watched with bookmarks, and a broken watch resumes from its
  resource version instead of listing again;
- `managedFields` and the other bookkeeping are dropped as they arrive;
- API discovery is cached per cluster on disk.

**Rows are drawn only where they are seen**, as every list of Groove already is.

## The agent's harness

**MCP tools, the same client, the same store.** The agent reads the resources, events, logs
and rollout history of what the session holds. A read of something already watched costs no
call and returns what the user sees.

**Reads are bounded.** A tool returns a tail of logs, a field, a summary — never a stream, and
never a whole namespace's YAML.

**Writes go through the queue** as the table above says: apply, patch, scale, restart,
delete, sync. A write outside the session's attachments is flagged on the sheet.

## Skills as resource actions

**A skill can declare the kinds it acts on**, as it declares the session kinds it belongs to
today: `groove-resources: pod, deployment`. Such a skill appears on the right-click menu of
a resource of those kinds, and nowhere else. Choosing it starts the skill in the session's
agent, with the resource — cluster, namespace, kind, name — as its argument.

"Debug" is then a skill like any other, and a team adds its own actions in Markdown, without
a change to Groove.
