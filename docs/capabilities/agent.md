# Agent

Run one agent per session, see what each does, and approve what it writes. Surface: the
rail, as [../design.md](../design.md#rail--mid-scope) draws it.

## Multi-agent

**One status per session's agent**, read from its hooks and its exit:

| Status | From | Until |
|---|---|---|
| working | `PreToolUse`, `PostToolUse`, `UserPromptSubmit` | `Stop` |
| asking | `Notification`; `PreToolUse` of `AskUserQuestion` | the next prompt or tool use |
| done, unseen | `Stop` | the session is selected on the rail |
| idle | at start; done and seen | the next prompt |
| exited | the pty exits non-zero | reopen |
| error | a failed action on the session | the next one that succeeds |

The done row says what changed since the user last looked: commits and files, from git.

**Attention class**, from the status: needs you — asking, an ask, exited, error; act when
you look — done unseen; moving — working; quiet — idle. Colour and motion follow
[../design.md](../design.md#rules).

**Every write the agent sends waits as an [ask](../glossary.md#ask)**, shown on its rail
row: the op and its subject, Approve and Review. With
[auto-approve](../glossary.md#auto-approve) on, the write is approved as it arrives and
recorded, except a cluster write, which always waits. The switch is per session; a new session takes the default from
Settings › Agent.

## MCP tools

**A tool is a controller function.** A read answers from the state; a write goes through
the approvals queue, and the call blocks until the user decides.

**Scoped to the session.** The tool server's URL carries the session id, and every tool
takes that session as its default.

**The tool descriptions are the one source** of how the agent writes. The core prompt and
the skills point to the tools; they never restate them.

**The agent reads a cluster only through Groove.** It never runs `kubectl` itself: the
launch denies it in Bash. It asks the `kube` tool, and Groove checks the arguments before it
runs the binary as an argument list, never through a shell. A read names a context the
session holds and a namespace it holds, takes only read verbs and flags, and never prints
a secret's data. Attaching or letting go of a cluster is a write, through the queue.

**The agent writes to a cluster only through `kube_patch` and `kube_delete`.** Groove reads
the object, tries the patch or the delete on that version with a server dry run, and queues
it with the diff of the object's YAML; a delete shows the whole object going. It waits for
the user even with auto-approve on. Once approved, it lands on the version shown, and an
object that moved since is refused. Enforced by
`a_patch_waits_with_its_diff_under_auto_approve_and_lands_once_approved`,
`a_delete_waits_with_the_object_going_and_lands_on_the_version_shown` and
`an_object_that_moved_before_the_approval_is_not_overwritten`.

**The agent runs a command in a pod only through `kube_exec`.** It names the pod, its
container and the command as an argument list; Groove runs `kubectl exec` with no shell,
no terminal and no input, 60 seconds unless the agent asks up to 300. The command waits for
the user as a cluster write does, shown on the Diff tab as a shell file, even with
auto-approve on, and a context set read-only refuses it. A character that does not print
shows escaped. The answer is its output, cut at 32 KiB, then its stderr; at its timeout
kubectl stops, and the answer says the command may still run in the pod. Enforced by
`a_command_waits_with_what_it_runs_under_auto_approve`,
`a_read_only_context_refuses_a_command_before_it_waits`,
`a_command_runs_in_its_pod_s_container_after_a_double_dash` and
`what_does_not_print_shows_escaped`.

## Conversation

**Read from the transcript, never stored.** A hook names the session's transcript; Groove follows
it from its start as it grows, and a new transcript starts the conversation again. The agent
runs headless, and a message sent from Groove's prompt goes over its stream. A pasted message shows as
the user's; one typed while the agent works waits until its turn takes it. A record kind Groove does not know is skipped.
Enforced by `a_new_transcript_starts_the_conversation_again_and_the_old_one_is_not_heard`,
`a_pasted_prompt_and_one_typed_while_the_agent_works_are_heard`,
`a_message_typed_while_the_agent_works_waits_until_its_turn_takes_it` and
`a_record_kind_groove_does_not_know_is_skipped`.

## Activity

**The timeline holds what a session did to its own work**: git — commit, push, pull; the
forge — an MR opened, written again, merged or closed, a CI result, a review, a thread opened;
repos and worktrees; the agent's cluster reads; each cluster write that lands, the user's or
the agent's, as its target and the fields it set, never their values; and the agent's turns, so the feed can say when
it worked.

**It does not hold the tools the agent ran on files.** A write or a shell command is the
agent's status, which the rail shows live, not its history.

A stored kind this version does not know is skipped rather than guessed at.

## Core prompt

**Passed per launch, never stored in the conversation.** It names the session and the
rules; its approval rule reads "a write may wait for a human".

## Skills

Core skills (`groove`), the user's own (`user`), and the team's (see
[../shared.md](../shared.md)). The agent reads and writes its own skills through tools;
a save asks like any write. A worktree row's state names the skill that fits it, and its
button sends that skill.

**A skill can act on a Kubernetes resource.** Its `groove-resources` lists the kinds it acts
on, `*` for every one. It then leaves the session's menus and stands on the right-click of a
resource's row of those kinds, in the list or in an Application, sent with the resource's
context, namespace, kind and name.

## Shared skills, knowledge, routines

As [../shared.md](../shared.md) describes them.
