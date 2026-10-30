# Design

Three surfaces, three scopes, and settings beside them. Information never leaks upward:
what belongs in a session does not reach the board, and what belongs on the board reaches
the rail only as its attention count.

| Surface | Scope | Holds | Capability |
|---|---|---|---|
| Board | large | every task, live or not, with its repos and MRs | Tasks |
| Rail | mid | the sessions actually open, and what their agents do | Agent › Multi-agent |
| Session | small | one session, in full | Sessions, Workspace |
| Settings | whole window | setup, providers, agent, appearance, preferences, keymap | Config |

## Rules

- **Colour means state. Motion means activity.** One colour per state: an ask is peach, a
  working agent is blue, CI and diff carry green and red. Text carries the colour, never a
  background. If it moves, an agent works.
- **A fact clears when the world changes, never when the user looks at it.** No dismiss.
- **Suggest, never auto-send.** The action of a state is the skill that fixes it, shown as
  a button. Groove never runs it by itself.
- **Everything is reachable from the keyboard.** The palette is the front door to every
  action. The mouse is a shortcut.
- **Selection is a background**: a rounded ground, one step above the hover's. A selected
  tab is held the same way. In the region that has the keys it takes the accent; elsewhere
  it stays neutral. The agent and a terminal show the keys by a solid cursor.
- **A region is one chord away.** `alt+r`, `alt+a`, `alt+o` and `alt+e` hand the keys to the
  rail, the agent, the workspace and the sidebar; the terminals keep their own chord. The
  workspace and the sidebar unfold first, and a press while they hold the keys folds them
  away. `ctrl+alt` with `h`, `j`, `k` or `l` moves the boundary of the region that holds the keys
  that way.
  Enforced by `alt_and_a_region_s_letter_hand_it_the_keys`,
  `a_second_press_folds_the_sidebar_away`,
  `alt_o_opens_the_workspace_then_gives_it_the_keys_then_folds_it` and
  `ctrl_alt_and_a_direction_move_the_focused_pane_s_boundary_that_way`.
- **Alt reaches the session, ctrl what is in it.** An `alt` chord works from every region,
  the agent and the terminals included: a palette, a region, the board on `alt+shift+b`,
  closing the session on `alt+shift+q`. A `ctrl` chord acts inside the region and leaves the
  agent's and the terminals' keys to them, except `ctrl+w` and `ctrl+tab`.
  Enforced by `alt_shift_q_closes_the_session`.
- **A list walks from the keyboard.** `ctrl+j` and `ctrl+k` step down and up any list, a
  palette's included, as the arrows do. Enforced by
  `ctrl_j_and_ctrl_k_walk_a_list_as_the_arrows_do`. In the sidebar the arrows step through the rows, Enter
  does what a click does, Space stages a changed file, Left and Right fold a folder, and
  `[` and `]` change the list.
  Enforced by `the_arrows_walk_the_changed_files_and_space_stages_one`.
- **The rail and the terminals answer too.** On the rail, Enter goes to the workspace and
  `a` and `v` approve or show the agent's ask. `alt+]` and `alt+[` step through the
  workspace's tabs, or the terminals' tabs while they have the keys. Enforced by `on_the_rail_enter_goes_to_the_workspace_and_a_and_v_answer_the_ask`.
- **Kubernetes walks from the keyboard too.** The kinds answer the arrows, a letter opens
  the kind palette on it. In the list the arrows walk the rows; Enter opens one on Describe, `y` on
  its YAML, `l` on its logs. `ctrl+tab` and `ctrl+shift+tab` walk the open files, objects or
  terminals, `ctrl+w` closes the one shown.
  Enforced by `the_list_s_rows_walk_and_open_on_describe_yaml_or_logs` and
  `in_a_terminal_ctrl_tab_steps_its_tabs_and_ctrl_w_closes_one`.
- **The board walks too.** `alt+1`, `alt+2` and `alt+3` open it, from anywhere, on its
  Live, Up next and Review columns; on the board `alt+r` is the rail. The arrows walk the column; Enter, or a click, opens the session and goes
  to it, `ctrl+Enter` or `ctrl+click` opens it and stays on the board.
  Enforced by `the_board_s_columns_walk_by_key_and_enter_goes_or_stays`.

- **Long lines wrap on a toggle.** With *wrap long lines* on (Settings › Appearance or the
  palette), the editor wraps each line to its width; the arrows step row by row, a click
  lands along the line, and nothing scrolls sideways. A note wraps in its card and while it
  is typed. Without wrap, the diff and the editor scroll sideways to keep the caret in view.
  Enforced by `down_steps_along_a_wrapped_line_before_the_next_one`.
- **The editor shows each scope's shape.** A block the syntax tree opens takes a guide down
  its indent, in the comment colour, and a caret in the column after the line numbers. A
  click on the caret folds the block under its line, which then says how many lines it
  hides; a fold the caret stands inside stays open, and a fold follows its block as lines
  come and go above it.
  Enforced by `a_click_on_a_block_s_caret_folds_it_under_its_line_and_says_how_much_it_hides`.
- **The word the caret rests in is grounded with every occurrence on screen**, after 125 ms.
  A selection takes a neutral ground, brighter than theirs; a search match keeps the accent.
  Enforced by `the_word_the_caret_rests_in_is_grounded_wherever_it_stands`.

## Limits

- **No accessibility layer.** Groove draws its own pixels and gives screen readers
  nothing. AccessKit (`accesskit_winit`) is the way in when it is needed. Every view
  draws through the kit, so the kit is the one place that would emit the nodes.
- **No input method.** Dead keys and Compose work; IBus and fcitx5 do not reach Groove.
- **No home-grown layout engine.** Boxes lay out with Taffy, through the kit. Lists and
  cell grids (terminals, code rows, list rows) keep fixed math inside the box Taffy
  gives them. Content shaped like a document opens in the browser or stays plain text.
  Groove never lays out flowing pages.

## Board — large scope

The board replaces the agent pane, the workspace and the sidebar; the rail stays. Three
columns, one per list, each scrolling on its own, on the deep ground. The page has no title.

**Header.** At the top, in the middle, the filter on its own pane of glass — `field:value`
tokens and bare words, with autocomplete — applied to all three columns at once. Just right
of it, a round **+** made of glass, which starts an explorer; the pointer lights its whole glass.
Filing the task at the provider is the agent's own write. Enforced by
`the_filter_stands_on_glass_at_the_top_middle_and_the_button_beside_it`.

| Column | Holds | Sort |
|---|---|---|
| **Live** | every session on disk, open or closed | last opened |
| **Up next** | tickets with no session, in the user's local order — the plan | the user's order |
| **Review** | the forge queue: MRs where the user is a requested reviewer, neither approved by them nor open in a session | updated |

Each column heading carries its count. Each item is a rounded card; no rule parts them.

**Live item.** One line: the kind icon, blue while the session is open, the title, then
the repo count and the worktree count when it is not one. A click opens the session,
which joins the rail. Right click: delete locally.

**Up next item.** A position number, title, priority and size as text. Drag to reorder. The order is Groove's and is never
written to the provider. A ticket that gets a session leaves the plan for Live.

**Review item.** Title and updated, then under them the project and MR number, the
author and where the reviewers stand. Open creates a review session, which joins the
rail. The column heading sorts by title or by updated; a second click turns the order
round. Up and down select an item, Enter opens it, Esc lets it go.

**Attention.** An item that needs the user gets one line under its title — the reason and
the age, in peach — and floats to the top of its column whatever the sort:

```
◎ shop-deploy !88          waiting 5d
⚑ Harden Groove…             changes requested · 2d
⚑ finops export              CI failed · 3h
```

The rules, each with an age and a threshold in Config › Preferences: a review waiting
longer than the threshold; changes requested; a run that failed and stayed failed for ten
minutes; approved and green but not merged for a day; due soon and overdue. The rail's
Board row carries the count of items with attention, in peach, and nothing else.

No timeline, no activity heatmap, no *blocked by*: the task tools draw those better.

## Rail — mid scope

The rail is a pane of glass floating inside its place, a small gap from the window's edges.
It holds only the sessions actually open, under three headings, each with its count: **needs you**,
**working**, **idle**. A heading with no session is not drawn. Under a heading the sessions
keep the order they were opened in; a session moves only when its state does. The Board row
above them carries the board's attention count when it is not zero. Enforced by
`each_row_stands_under_its_state_in_the_order_opened`.

Needs you holds the agents that ask, have exited or failed. Working holds the agents at work.
Idle holds the rest, the done ones among them: done keeps its green, and opening it moves
nothing. Up and down walk the rows in the order
drawn. Enforced by `the_arrows_walk_the_rows_in_the_order_drawn`.

Each row:

1. **State dot** — peach while it asks, green while done and not seen, red on an error, blue
   while working, grey when idle, hollow once exited.
2. **Type icon** — task, review, explorer, routine.
3. **Title.**
4. **Agent status** — `working`, `asks to commit`, `done`, `idle`, `exited`, `error`. One
   line, truncated, never wrapped.
5. **Relative time, right-aligned** — how long the agent has waited, or since it finished.

```
 ▤  Board                                   2
 NEEDS YOU · 1
 ● ⚑  TASK-51  auth refresh                2m
      asks to commit
 WORKING · 1
 ● ◎  review-3  shop argo                 now
      working
 IDLE · 2
 ● ⚑  fix/keys-50                          6m
      done
 ● ⚑  TASK-48  finops export               3h
      idle
```

Nothing else: no CI, no MR state, no worktree count. Rows stand inset from the rail's sides
on rounded, see-through grounds, with no rule between them, as the palette's rows do: the
selected one stronger, the one under the pointer lighter. Headings, rules and the footer keep
the same inset. A row takes a tinted ground only while its agent asks, in peach, or is done and
not yet seen, in green. Enforced by
`the_selected_row_stands_inset_on_a_rounded_held_ground_and_the_hovered_one_lit`.

A row carries no button. An ask's text is peach, and a click on its row opens the session
with the review sheet on the first ask. Enforced by
`a_click_on_a_row_that_asks_opens_the_session_on_its_review`. With auto-approve on for the
session, nothing asks. A working status is blue and its glyph moves.

Opening a session starts its agent. Closing a row ends the agent and removes the row; the
session stays as it is on disk.

**Routines.** A standalone routine's session stands under its own heading, `Routines · N`,
folded by default, below the other sessions. Its row offers a round glass play button, which
runs the routine now, while no run is on it. Selected, it shows its agent pane alone, full width. It never
appears on the board.

**Feed.** Below the rows, the event log of the opened sessions, newest first, filterable to
the selected session: agent turns, commits, pushes, MR events, CI results, notes.
Monochrome, no motion, collapsible. Asks are never in the feed; they live on the row.
Back-to-back events of one kind from one session fold into one row with a count; a click
opens the run to one line per event.
It shares the rail's glass with the rows, set apart by a gap, not a rule. Two icon tabs at its
heading's end pick the session in hand or every session. Each line has its mark in a round. An
open run and its lines stand on one held ground. An error lies on its colour's tint.
Enforced by `back_to_back_events_of_one_kind_fold_into_one_row`,
`the_scope_tabs_pick_the_session_in_hand_or_every_session`,
`an_open_run_and_its_lines_stand_on_one_held_ground` and
`an_error_lies_on_its_colour_s_tint`.

## Session — small scope

The rail, then one ground for the agent with the workspace's strip of glass on it.

| Part | Holds |
|---|---|
| **Rail** | opened sessions, the feed |
| **Agent pane** | a top bar · the agent's pty, or its conversation and its prompt |
| **Strip** | header · info line · the selected tab · the sidebar |
| **Sidebar** | inside the strip: contextual list for the selected tab; folds away |
| **Terminals** | a drawer under the strip: a capsule while folded, the grids open |

**The agent pane draws the conversation** when *conversation view* is on (Settings › Agent);
off, it shows the pty. The agent's rows stand at the left and the user's bubbles at the right,
each side at most nine tenths of the width. One sent from the
prompt, or typed while the agent works, waits faint after the rows until the transcript says it.
Thinking, reads, commands and MCP calls fold into one row: a coloured mark and a count for each,
a line of what its last step does, kept current, and the time the group took at its right. Its card holds the last five commands and calls,
each one line cut with `…`; a click opens it to every one whole and wrapped, with what it gave back.
An interrupt is a row of its own, and no group turns after it. While the agent works, the last group's mark
spins and its time runs, and a group of its own stands past the last record until one comes. A
write is one row: its file links to the hunk in the Diff tab, with its `+N -N`. A table is laid in
columns. A drag selects text and copies it on release; a double click takes a word, a triple click
its line. Ctrl and a click follow a URL, or open a path in the Files tab at its line.
The prompt is glass a bar's height above the pane's foot, as the workspace stands over the
terminals, centred and no wider than the palette. The conversation scrolls on under it and the
bar, and at the top under the top bar. The prompt edits as a note does, grows to eight
rows and then scrolls. Enter sends it to the agent, shift+Enter breaks the line.
Up in an empty prompt brings back the user's last message, and each Up the one before; Down
steps forward, and past the newest empties the prompt.
Enforced by `the_conversation_shows_in_place_of_the_terminal_while_settings_turn_it_on`,
`a_click_on_a_command_card_shows_every_command_whole_with_what_it_printed`,
`while_the_agent_works_past_its_last_record_a_turning_glance_says_so`,
`an_interrupt_ends_the_turn_and_no_glance_turns_after_it`,
`up_in_an_empty_prompt_steps_back_through_what_was_sent_and_down_returns`,
`a_table_keeps_its_rows_and_each_span_its_cell`,
`a_drag_over_the_conversation_copies_what_it_holds`,
`a_double_click_takes_a_word_and_a_triple_click_its_line`,
`ctrl_and_a_click_follow_a_link_or_open_a_file_at_its_line`,
`the_prompt_keeps_its_spaces_and_selects_and_copies_as_a_note_does`,
`a_double_click_in_the_prompt_takes_its_word`,
`the_prompt_follows_its_caret_past_its_tallest_and_the_wheel_scrolls_it`,
`enter_sends_the_prompt_to_the_agent_and_shift_enter_breaks_its_line` and
`a_click_on_an_edit_s_file_opens_its_hunk_in_the_diff`.

**The strip folds to its header.** A click on the open tab folds it: the header alone stays
at the top right, a capsule as narrow as its title and tabs, and the agent takes the
ground's whole width. A click on any tab opens it again on that tab. The agent's terminal
takes its new size at once. Folding the terminals leaves a folded strip folded. Enforced by
`a_click_on_the_open_tab_folds_the_strip_and_widens_the_agent`,
`any_tab_opens_the_folded_strip_on_itself`,
`the_folded_strip_is_a_capsule_as_narrow_as_its_title_and_tabs` and
`folding_the_drawer_leaves_the_strip_folded`.

**The terminals rise from the foot.** Their drawer is a pane of glass under the strip.
Folded, it is a capsule at the bottom right, as narrow as its bar. Open, it rises to the
height the user dragged and the strip ends a gap above it; with the strip folded it fills
the column under the strip's header, and the agent gives it the room until it folds again.
Enforced by `the_folded_drawer_is_a_capsule_as_narrow_as_its_bar`,
`an_open_drawer_rises_its_height_and_the_strip_ends_above_it` and
`an_open_drawer_fills_the_column_beside_the_agent_while_the_strip_is_folded`.

The rail, the agent pane and the sidebar each have a width the user drags, kept between
runs. The strip takes what is left, folded or open. A boundary moves the two parts it stands
between and nothing else. The sidebar folds from the info line's far end or with
`alt+e`.

### Workspace

**Header.** One line at the strip's top: the type icon and the title, then the tabs. Each tab
is a rounded row on the glass, held while open, lit under the pointer; the terminals' tabs
share it. Finishing the task and deleting the session here are palette rows. A button that
is one icon alone is a circle. Enforced by
`a_task_or_a_review_is_deleted_here_from_the_palette_and_an_explorer_is_not`.

**Info line.** Under the header while the strip is open: the selected repo and worktree,
which every tab and the terminals follow, with the worktree's MR and CI and a refresh
button, and the sidebar's fold at the far end. A git mark leads it. It only tells: `alt+w`
opens the palette on every worktree as `repo/branch`, the best fuzzy match first, and Enter
selects one. The Kubernetes tab's own first line, led by a cluster mark, holds its scope:
the contexts shown, a lone one in its hue, then the namespaces. It only tells: `alt+c`
opens the context panel and `alt+n` the namespace panel, from any tab, where the palette
stands, with a search and the first line stood on. The context panel holds the session's
contexts, a check to show one and a × to detach it, then what it could attach; a pick
there attaches the whole cluster. The namespace panel lists `context/namespace` for every
context held, `context/*` for the whole cluster, the best fuzzy match first: space shows
one more, Enter shows one alone and gives the list the keys. Either choice turns to the
Kubernetes tab. A title or a
label too long for its line is cut with an ellipsis; no line wraps; a tab's status badges
keep their room before the cut, and a badge with no room left is dropped.

**Tabs.** `Overview · Diff · Files · Kubernetes`, the last while the session holds a cluster.

| Tab | Shows | Sidebar |
|---|---|---|
| overview | sections, each a heading over its own card, inset from the panel's sides: the task's properties on one line — status, priority, dates, hours — a missing one left out; the repos, each with its worktrees as rows read from the left — the branch, git's counts, then the MR's number, verdict, CI status and notes, zero counts and an absent MR omitted; the clusters it holds, each context in its hue with its namespaces after it; the body as text. Enforced by `each_section_stands_in_its_own_card_inset_from_the_panel` | folded |
| diff | the whole change as one stream, inline or split, notes inline, editable on its new side | one search field, always there, for a path or for text, a click on the other term's name switching it; then a strip that picks the list, its rows lit and held on rounded grounds as the rail's are (enforced by `one_search_field_always_stands_over_the_strip_and_switches_its_term`): files — the files that changed, a click scrolls the stream to one, with stage, unstage and discard by right click; commits — the branch's own log, a commit opens its change read-only; notes — the session's annotations and threads, open, resolved or all, under the file each stands on and the MR's own last, two lines a note; a note scrolls to its line, a resolved one opened |
| files | the open files, one tab each with a dot while it owes the disk; the active one on the code surface, marked where the change touched it | the search bar, then a strip that picks the list: notes — the diff's own list, a note opening its file at its line; files — the whole worktree as a tree with the path operations by right click; a click opens a file in the preview tab, in italic, which the next click replaces; a double click or the first edit keeps it. A found line opens at the line. A tab's right click offers close, close others and close all, which leave an unsaved file open; a middle click closes it |
| kubernetes | the list tab, titled by its kind and its count, and a tab for each object opened from it; [Kubernetes tab](#kubernetes-tab) draws both | the kinds the contexts serve, under headings that fold |

**One code surface.** The Files tab's editor and the Diff tab's two views are one surface,
which the gutter holds together.

| Mode | Shows | Gutter |
|---|---|---|
| files | the file as it is now | a mark on every line the change touched |
| inline | the change as one column | the old and the new line number |
| split | the old beside the new | one number per side |

An added or a removed row wears one tint across its gutter and its text, and its numbers
lean to its colour. Enforced by
`a_changed_row_wears_one_tint_across_its_gutter_and_its_numbers_lean_to_it`.
In split, a side with no line is hatched, gutter included.

Over the diff, a band holds the whole change from the left — how many files, what it added
and removed — then the inline and split switch and the base and working modes as rounded tabs. Over a file, the band names the file from the left, then what stands beside it: unsaved,
changed, faults, servers (enforced by `a_file_s_band_reads_from_the_left_its_counts_right_after_its_path`). No rule stands between the gutter and the code; a run of unchanged lines stands between two
rules, its arrows icon buttons. A file's head is a rounded row inset from the sides. Enforced by
`no_rule_stands_between_the_gutters_and_the_code` and `a_gap_reads_as_a_band_across_the_rows`.

The new side is editable in every mode. A caret lives in the document, never in a row, so
it stays where the user left it while the rows move under it. A removed line belongs to the
old document and takes no caret. **Where the caret is** is two rules, above its row and
below it, across the surface — never a ground.

**Word diff.** A run of removed rows and a run of added rows of the same length pair one
for one, and each row shades the words its pair does not have. A line too long, or a pair
too far apart, is left to the row's own ground.

**Editing is modeless.** Keys do what they do everywhere else: characters type, arrows
move, shift holds, `ctrl+c/x/v` carry, `ctrl+z` undoes, `ctrl+s` writes. Enter opens the
next line at the indent of the one it left. Tab writes what the language's own formatter
writes — a tab in Go, two spaces in YAML and Markdown, four elsewhere.

**Search.** Two live rows in the sidebar: a path and a text search of the worktree. In the
surface, `ctrl+f` floats a glass bar at the top right of the rows, with the count at its
end; enter and shift+enter step to the next and the previous match, esc ends it and leaves
the caret on the match. `ctrl+p` opens a file by its path, `alt+f` the palette on the
worktree's files. Enforced by `enter_and_shift_enter_step_the_matches_and_wrap` and
`alt_f_opens_the_palette_on_the_worktree_s_files`.

**Finding your way in a big change.** Three answers, and none of them is a bigger tree:

- **The path, once.** The file list strips the whole change's common root and shows it
  once, then collapses every directory chain with a single child. A file reads as its name
  first and the rest of its path behind it, dimmed. Where the name carries no meaning,
  `mod.rs` and its like, the directory is the name.
- **Where am I.** In the diff, each file opens on a head two rows tall: its name, its
  folder dimmed after it, then its counts and its read box at its end. The head of the
  file the rows stand in pins at the top, over a soft shadow, until the next file's head
  pushes it out. In the file view, the scopes
  the visible rows sit in pin instead, stacked as they nest, read from the syntax tree.
- **What is left.** One column down the edge holds the whole change: a band per file, its
  height its diff's length, its hunks as marks, the viewport as a lens that drags. A file
  marked read dims. What is left is what is still bright.
  A file is marked read by the user, never by scrolling past it. The marks belong to the
  session and outlive the window.

**Manual section.** Where the user acts by hand, as the agent pane is where the agent acts.
Terminals for the session, opened in its own directory, splittable, resizable, collapsible;
its bar alone when collapsed.

**Git.** The palette's Git group acts on the selected worktree: push, pull, the MR's own
actions, and discard every change. No box in the window commits: the agent or a terminal
does.

**One deep ground, glass over it.** The deepest ground the theme has holds the session
under the agent and the strip, the terminal widget, the board and Settings. What stands
over it — the rail, the strip and its sidebar, the board's columns, Settings' sections —
stands on glass, with no ground of its own. A note floats over the rows it takes as one pane of glass, on the layer
the sticky file head uses, its replies and its buttons ruled off on it; a note being written
does the same, its field a rounded field. Enforced by
`a_read_note_floats_over_its_rows_as_one_pane_of_glass` and
`a_note_being_written_stands_on_glass_as_a_read_one_does`.

**Glass only where it floats.** The rail, the workspace's strip, the terminals' drawer, the palette, every
menu, the cluster scope panel, the board's filter offers, the language servers' popup, the agent's
prompt are glass: the work below shows through them, frosted, magnified and bent at the rim. The completion
list stays solid. A row on glass takes a rounded, see-through ground under the pointer and when selected. A button on
glass is a pill of glass, lit whole under the pointer. Enforced by
`the_rail_floats_as_a_pane_of_glass_inside_its_place`,
`the_strip_stands_as_glass_on_the_agent_s_ground`,
`the_palette_is_glass`,
`a_menu_is_glass`,
`the_panel_stands_on_glass`,
`the_hover_popup_is_glass`,
`the_completion_list_stays_solid` and
`a_palette_row_under_the_pointer_takes_a_rounded_see_through_ground`.

**Three grounds a row can take, and they never share a value.** Under the pointer is the
quietest, a selected row is stronger, and what a click acts on is stronger again. Where the
caret is, two rules stand instead of a ground: a place is not a state. The row a list stands
on — the open file, the shown commit, the kind listed — has the two rules over the held
ground, the accent dimmed into it. The rail is the exception: see below.

**The palette keeps its groups while it filters.** Each group runs under its heading, the
group with the best match first. A first word that starts a group's name keeps that group
whole and on top: `go` lists every Go entry, `git pu` finds Push.
Enforced by `a_group_named_first_comes_first_and_whole`.

**A step names what it asks.** An entry that asks several questions shows the current one,
dimmed, before the caret: `repo`, then `branch`, then `target`. Backspace on an empty field
takes the last answer back into it, and on the first question returns to the entries.
Enforced by `backspace_on_an_empty_query_takes_the_last_answer_back`.

**A session, a worktree, a kind and a namespace are one chord away.** `alt+s` opens the
palette on the other open sessions by title and id, `alt+w` on the session's worktrees as
`repo/branch`. `alt+k` opens it on the kinds the session's
contexts serve, each with its short names and its group; a short name typed whole comes
first, and Enter lists the kind. `alt+n` opens the namespace panel, `alt+c` the context
panel.
Enforced by `alt_s_picks_another_open_session_by_its_title_or_its_id`,
`alt_w_picks_a_worktree_by_repo_and_branch_ranked`,
`alt_k_picks_a_kind_by_its_name_its_short_name_or_its_group_and_lists_it`,
`alt_n_then_a_ranked_query_and_enter_shows_that_namespace_alone_and_space_adds_one` and
`alt_c_opens_the_context_panel_and_again_shuts_it`.

**`#` searches files.** A query that starts with `#` lists the selected worktree's files, a
match in the file name first, and opens the one picked on the Files tab.
Enforced by `a_hash_searches_the_worktree_files_and_opens_the_one_picked`.

**Git and the MR go through the palette.** Push, pull, discard every change, and
create, update, comment on, approve or close the MR are palette rows of the selected
worktree. No box in the window holds them.

**A row offers, it never surprises.** Pointing at a file shows one word at its end — stage,
or unstage — where its counts were. Everything else is behind the right button.

**A question takes the place of what asked it.** Discarding one file turns its row into the
question; discarding every change turns the info line into it. Two answers at
the end, no modal, and nothing destructive within a click of something ordinary.

### Kubernetes tab

- **List.** The list tab, titled by its kind and its count, over the rows of every context
  and namespace in scope: the server's Table columns `kubectl get` shows, behind a cluster
  column and a namespace column where those tell rows apart; a status and a ready count in
  the colour of how the object stands. A time cell, the age or a last schedule, counts up
  live; the age from the object's creation. A header sorts by its column, numbers and ages
  by value, and a second click turns the order round; its edge drags the column's width,
  kept for the kind. `ctrl+f` opens a find bar over the rows: fuzzy name words —
  `bil-eu-st` keeps `billing-eu-staging` — and `label=value` words the server filters
  on. `alt+c` opens the context panel, `alt+n` the namespace panel, `alt+k` the kind palette. No rule stands under the header; a row under
  the pointer or selected stands on a rounded ground, as the rail's rows do. The list's tab
  and each open object's are glass tabs as the Files tab's are, an object's with its ×, no
  rule under them; the find bar is a rounded field.
- **Object.** A row opens its object in a tab of its own beside the list, `describe ·
  yaml`, and `logs` for a pod. The tab reads the object, its events and its owners by name,
  and a pod's usage every 15 s, while it shows. As YAML it is the file editor's own view of
  the object, read-only. Its heading holds the object's status badges, then its views as
  glass tabs at the end, and no rule stands under it.
- **Describe.** Described, it is one column of sections, each folding under its heading:
  conditions, a summary of facts a line each, a pod's containers, labels, annotations,
  relations and events. Its relations are the list's table under headings that fold (what
  owns it with its ready count, what it uses and how — mounted at a path, read as env by a
  container, pulling images —, what uses it); its containers the list's table (state,
  ready, restarts, the last end, CPU and memory each a bar of the usage against the limit
  with the request a tick, both told under the pointer) whose rows unfold into the image,
  probes, ports and mounts a line each, the ones not ready unfolded. Annotations, relations
  and events start folded. A name, an address, an image, a label or an event's message
  copies on a click: the copy mark shows on hover and turns to a green tick once copied. A
  link copies the same way; `ctrl`+click opens its object. Each section stands on a
  rounded glass card, its heading lit whole with its caret under the pointer, with no rule
  between the sections or inside them. A copy under the pointer lights on a rounded glass
  ground, and a usage's tip floats on glass.
- **Logs.** Its logs are the file editor's view too, read-only, at the small text size: a
  tab per container and one for all of them, the current run or the previous one, the last
  5 minutes to 24 hours or from the start, each line after its clock; a line naming an
  error is red, a warning yellow. The view follows the last line until the wheel moves it
  up; the footer then counts the lines below and follows them again on a click. A stream
  holds 10 MB, its oldest lines going first, and picks up after a break from its last
  second. Its containers, runs and range are glass tabs over the lines, and no rule
  stands over or under them.
- **Argo CD.** Argo CD's Applications list their project, sync, health, auto-sync and
  revision. An Application's describe holds its last sync, red when it failed and blue
  while it runs, its heading included; its conditions; where it syncs to and how; and its
  sources, a line each with the revision it synced to. The summary names the context the
  destination stands on: the Application's own for the in-cluster address, else the one
  added whose server matches, read once from the kubeconfig without a sign-in check. A
  resource's row opens its tab there. A git source's address, ctrl+clicked, opens the
  files on its repo, selected in the session or added from the pool, cut from the branch
  the source follows when it is one; its synced commit, ctrl+clicked, shows in the diff as
  a commit of the list does, fetched first when the clone lacks it. Its `resources` view
  filters by what needs attention (out of sync, to prune, or failed in the last sync), out
  of sync, syncing or all, each in its colour with its count; it opens on what needs
  attention, or on the sync while one runs. Its filters are glass tabs. Both it and syncing
  are the list's table, its headers sorting each group's rows and its edges dragging widths
  kept for both; no rule stands between a group's heading and its rows. Resources stand by
  kind, the kinds needing attention first; under all, a kind with nothing to see stands
  folded. Syncing shows the sync's line, rounded on its colour, over a rounded bar that
  fills resource by resource, then the PreSync hooks, each wave in order with its Sync
  hooks on a badge, and the hooks after, each step applied, applying, waiting or failed; a
  stage done stands folded.
- **Sidebar.** The kinds the contexts serve, indented under headings that fold: workloads,
  network, config, storage, RBAC and other for Kubernetes' own, one a CRD group by its
  name, then cluster for the built-in kinds not namespaced. ↑ and ↓ step through the
  headings and the kinds; Enter folds a heading or lists a kind. No counts, no health
  marks. A heading or a kind under the pointer lights on glass, and the kind shown stands
  on a rounded ground.

### Review sheet

An ask opens a sheet over the work half — workspace and sidebar. The agent
pane stays visible. One frame for every op: the repo and the branch, then the text of the
write. No diff and no file list. A push goes to the worktree's own branch and nowhere else.

| Op | Text shown |
|---|---|
| commit | the message: title and body |
| push | each commit's title and body |
| MR create, update, close | the MR's title and body |
| task property | the task's title, the property, before → after |
| task body | the task's title and the body |
| task finish | the task's title and the worktrees torn down |
| discard | the files |

**Approve** in peach, **Refuse**. No editing in the sheet. Esc closes it without deciding.

## Settings

Config's surface. Takes the whole window, the rail included; Esc or *back* returns to where
the user was. Reached from the rail's footer and the palette. A section list on the left,
on a floating pane of glass as the rail's — Setup, Providers, Clusters, Agent, Appearance,
Preferences, Keymap — under the title, *back* and a search field: typing filters every
section to the matching rows, each shown with its section. The selected section's form on
the right: labels left, controls right as glass buttons, one row per setting, each group a
rounded card under its name, paths and ids in mono. Clusters alone is a table on its card,
since each context holds several settings.

**Setup opens on its own once.** At start Groove checks the environment; when a required
program is missing, claude is signed out or no config file stands, Settings opens on Setup,
once a launch, unless the user acted first. A section that still needs something shows its
count in the list. Enforced by `setup_opens_once_when_the_first_check_finds_something_left`.

| Section | Rows |
|---|---|
| Setup | environment check — git, gh, glab, curl, kubectl, jq — each with its version and a mark · claude login · the config file, the state database and the worktree root as paths · next, Providers, while no task source stands |
| Providers | each task source, Notion or GitHub: its toggle with its gaps counted, where it is read from, then its fields, its status values and its priority values under their own headings, each a picker with what its gap costs beside it · forge tokens, gh and glab, present or missing |
| Clusters | a table, not rows: each added context with its hue and name, its read-only toggle, its login and remove; only the toggles and buttons act · then the kubeconfig's other contexts, each with add |
| Agent | auto-approve default · conversation view · the shared repo, by URL and branch · the skills — core, the user's own, shared — each with its toggle · the routines — pause all, how many run at once, each routine and its triggers |
| Appearance | theme — Latte, Frappé, Macchiato, Mocha · UI font: Inter · mono font: IBM Plex Mono, JetBrains Mono, Lilex · three sizes: interface, editor, terminal |
| Language servers | each server the config names: its switch, the languages it serves, its program and what the check found it to be |
| Preferences | attention thresholds — review waiting, due soon and approved unmerged in days, a failed run in minutes · the forge poll's interval and its stale threshold |
| Keymap | every action with its chords; a chord is rebound by pressing it |

Every change saves to the config file on the spot. No save button.
