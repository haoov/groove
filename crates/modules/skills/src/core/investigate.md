---
name: investigate
description: Investigate a Kubernetes resource and report what is wrong with it, or that nothing is.
groove-resources: "*"
---

# Investigate a resource

Find out whether this resource has a problem, why, and since when, then report it. The
resource is in the arguments: context, namespace, kind and name. Read the cluster with the
`kube` tool only.

- Check what the team already knows: its shared knowledge, and any runbook or post-mortem
  for this service. It can end the search early.
- Work from hypotheses. Each read should confirm or rule one out; say which.
- Start at the resource and widen only as far as the evidence leads: what owns it, its
  related objects, its namespace, its node, then the cluster.
- Look back in time, not only at now. Restarts, failed Jobs and old events point to
  incidents that are over. Find when it started.
- Compare. A healthy sibling, another namespace or another cluster tells "broken here" from
  "broken everywhere". Attach what you need with `attach_cluster`.
- A read that is refused for permission is a finding. Report it and leave that path.
- Read only. Never make a fix.

Report briefly:
- healthy or not, and since when;
- the cause and the chain of events, with the lines that show it;
- how sure you are, and what would settle it;
- a next step, with the exact command the user could run;
- anything else wrong you noticed, and what you could not check.
