# Clusters

A session can hold Kubernetes clusters. You then browse their resources, read pod logs
and follow Argo CD from the session, and the agent can read them.

## Where Groove finds contexts

Groove reads the kubeconfig contexts at launch:

- each file in `$KUBECONFIG`, merged as `kubectl` merges them, when the variable is set;
- else `~/.kube/config`.

It supports exec plugins such as `kubelogin`, client certificates and tokens. Restart
Groove after you add a context to your kubeconfig.

## Add a cluster to Groove

Open Settings › Clusters. The table shows the contexts Groove knows, then the other
contexts of your kubeconfig, each with **add**.

For each added context, Groove shows:

- its **hue**, the color that marks the cluster in a session;
- its sign-in state. When the cluster refuses you, sign in from a terminal, for example
  with your `kubelogin` command.

## Attach a cluster to a session

A session uses only a cluster you added in Settings. In the session, open the palette
and choose **Attach cluster**, then a namespace or the whole cluster. **Select
namespace** (`ctrl+shift+n`) changes the namespace. **Detach cluster** removes it.

The agent can also attach a cluster. That is an ask, so it waits for you.

## The Kubernetes tab

`alt+shift+4` opens the Kubernetes tab of a session that holds a cluster.

- The sidebar lists the kinds, custom resources included. `ctrl+shift+p` searches them.
- The table shows the columns the server gives for that kind.
- `ctrl+f` filters the rows. It takes name words and `label=value` terms.
- Open a row to get a tab with **describe** and **yaml**, and **logs** for a pod.

## Logs

The logs of a pod show:

- a time range: the last 5 minutes, 15 minutes, 1 hour, 6 hours, 24 hours, or from the
  start;
- one tab per container, and **all**;
- the current run or the previous run.

Error lines are red and warning lines are yellow. Groove keeps the last 10 MiB.

## Argo CD

Groove reads Argo CD `Application` objects from the cluster itself. It needs no Argo CD
URL or token, only the Application custom resource in an attached cluster.

- The list shows the name, project, sync status, health, auto-sync and revision.
- A filter shows the applications that need attention, are out of sync, are syncing, or
  all of them.
- Describe shows the last or running sync, the conditions, the destination and the
  sources. A failed sync is red and a running sync is blue.
- `ctrl`+click a git source opens the repo. `ctrl`+click a synced commit shows it in
  the diff.
