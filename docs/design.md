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
  background. If it moves, an agent works, or the user just opened it: what opens
  over the window — the palette, a menu, a scope panel — fades in over 140 ms, the palette dropping
  into its place; closing is a cut. A panel folds and opens at once. Nothing moves by itself, and
  with *reduce motion* on nothing moves at all. Enforced by `a_fold_stands_at_once`,
  `the_palette_fades_in_as_it_opens_then_stands_whole` and
  `with_reduced_motion_the_palette_stands_at_once`.
- **A fact clears when the world changes, never when the user looks at it.** No dismiss.
- **Suggest, never auto-send.** The action of a state is the skill that fixes it, shown as
  a button. Groove never runs it by itself.
- **Everything is reachable from the keyboard.** The palette is the front door to every
  action. The mouse is a shortcut.
- **Selection is a background**: a ground one step above the hover's, neutral wherever it
  stands. A selected tab is held the same way.
- **Tabs are segments of one box.** A set of tabs stands edge to edge inside a single hairline
  box; a tab alone takes the box to itself. The selected one is held on the neutral ground,
  square, the one under the pointer lit; the others' labels are faint. A label is small and mono,
  a word in capitals — `DIFF`, `ALL` — and a name as it is given: a file, a terminal, an object,
  a container. Enforced by `a_set_of_tabs_stands_edge_to_edge_in_one_hairline_box`.
- **Focus is a ring**: the region that has the keys — the sessions' window, the sidebar, the
  terminals — stands in a thin ring in the accent, read as half its strength on either theme.
  The middle has no edge to ring: the agent's prompt, open, takes the ring, and the code its
  caret. A terminal shows them by a solid cursor. Enforced by
  `the_ring_stands_around_the_region_that_has_the_keys_and_nowhere_else`,
  `the_open_prompt_stands_in_the_ring` and
  `a_selection_stays_neutral_in_the_region_that_has_the_keys`.
- **A region is one chord away.** `alt+r`, `alt+a`, `alt+o` and `alt+e` hand the keys to the
  rail, the agent, the middle and the sidebar; the terminals keep their own chord.
  `alt+shift` with the arrows walks the rail, the middle and the sidebar. The sidebar unfolds
  first, and a press while it holds the keys folds it away. `ctrl+alt` with `h`, `j`, `k` or
  `l` moves the boundary of the region that holds the keys that way.
  Enforced by `alt_and_a_region_s_letter_hand_it_the_keys`,
  `a_second_press_folds_the_sidebar_away`,
  `the_chord_walks_the_panes_left_to_right`,
  `alt_o_gives_the_keys_to_the_middle_and_alt_e_to_the_sidebar_then_folds_it` and
  `ctrl_alt_and_a_direction_move_the_focused_pane_s_boundary_that_way`.
- **Alt reaches the session, ctrl what is in it.** An `alt` chord works from every region,
  the agent and the terminals included: a palette, a region, the board on `alt+shift+b`,
  closing the session on `alt+shift+q`. A `ctrl` chord acts inside the region and leaves the
  terminals' keys to them, except `ctrl+w` and `ctrl+tab`.
  Enforced by `alt_shift_q_closes_the_session`.
- **A list walks from the keyboard.** `ctrl+j` and `ctrl+k` step down and up any list, a
  palette's included, as the arrows do. Enforced by
  `ctrl_j_and_ctrl_k_walk_a_list_as_the_arrows_do`. In the sidebar the arrows step through the rows, Enter
  does what a click does, Left and Right fold a folder, and `[` and `]` change the list.
  Enforced by `the_arrows_walk_the_changed_files_and_enter_hands_over_the_keys`.
- **The rail and the terminals answer too.** On the rail, Enter hands the keys to the middle and
  `a` and `v` approve or show the agent's ask. `alt+]` and `alt+[` step through the
  session's tabs, or the terminals' tabs while they have the keys. Enforced by `on_the_rail_enter_hands_the_keys_to_the_middle_and_a_and_v_answer_the_ask`.
- **Kubernetes walks from the keyboard too.** In the list the arrows walk the rows; Enter opens one on Describe, `y` on
  its YAML, `l` on its logs. `ctrl+tab` and `ctrl+shift+tab` walk the open files, objects or
  terminals, `ctrl+w` closes the one shown.
  Enforced by `the_list_s_rows_walk_and_open_on_describe_yaml_or_logs` and
  `in_a_terminal_ctrl_tab_steps_its_tabs_and_ctrl_w_closes_one`.
- **The board walks too.** On the board `alt+a`, `alt+o` and `alt+e` take its Live, Up next
  and Review columns, and `alt+r` the rail. The arrows walk the column; Enter, or a click, opens the session and goes
  to it, `ctrl+Enter` or `ctrl+click` opens it and stays on the board.
  Enforced by `the_board_s_columns_walk_by_key_and_enter_goes_or_stays`.

- **Long lines wrap on a toggle.** With *wrap long lines* on (Settings › Appearance or the
  palette), the editor wraps each line to its width; the arrows step row by row, a click
  lands along the line, and nothing scrolls sideways. A note wraps in its card, and a reply
  while it is typed. Without wrap, the diff and the editor scroll sideways to keep the caret in view.
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

The board replaces the session's bar, middle and sidebar; the rail stays. Three
columns, one per list, each scrolling on its own, on the deep ground. The page has no title.

**No header.** The columns' headings stand at the top; the board has no filter and no
button. An explorer starts from the rail or the palette. The columns stand clear of a bar's
height above and below, and across the rail's column once both its windows are folded.
Enforced by `the_board_opens_straight_on_its_columns_headings` and
`the_board_keeps_a_bar_s_room_above_and_below_and_with_the_rail_folded_takes_its_width`.

| Column | Holds | Sort |
|---|---|---|
| **Live** | every session on disk, open or closed | last opened |
| **Up next** | tickets with no session, in the user's local order — the plan | the user's order |
| **Review** | the forge queue: MRs where the user is a requested reviewer, neither approved by them nor open in a session | updated |

Each column heading carries its count. Each item is a sunk card as the overview's are, the
band's ground inside a hairline edge; no rule parts them. Enforced by
`a_board_item_stands_on_a_sunk_card_as_the_overview_s_do`.

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

The rail is two opaque windows inside its place, a small gap from the window's edges
and from each other: the sessions' above, the feed's below. Nothing else stands in it; Settings
opens from its chord and the palette. Enforced by
`the_rail_stands_as_two_opaque_windows_inside_its_place` and `the_rail_holds_no_settings_row`.
The sessions' window is headed by its name and how many sessions are open, with a `+` that
opens a new explorer; a rule sets the header and the Board row apart. Enforced by
`the_sessions_window_is_headed_by_its_name_and_count_and_its_plus_opens_an_explorer`.
A click on the header folds the window to it, as the feed folds to its own: the feed takes
the column up to its header, nothing left to drag between them; a second click opens it as it was. `alt+r`
opens it and hands it the keys; pressed while it has them, it folds it and hands them to the
agent, as the sidebar's chord does. Enforced by
`a_click_on_the_sessions_header_folds_the_window_to_it_and_the_feed_takes_the_column`,
`both_folded_the_rail_is_its_two_headers`,
`the_rail_s_chord_opens_the_folded_sessions_and_hands_them_the_keys` and
`the_rail_s_chord_on_the_rail_folds_the_sessions_and_hands_the_keys_to_the_agent`.
It holds only the sessions actually open, in two sections the user sorts by hand, each
headed with its count: **active** and **paused**. A section with no session is not drawn,
except while a row is carried. A new session lands on top of active. A row stays where the
user put it: it never moves because its agent's state changes. The section and the rank
persist. Enforced by `rows_keep_the_user_s_order_whatever_their_state_and_paused_ones_stand_below`
and `a_new_session_lands_on_top_of_active`. The Board row above them carries the board's
attention count when it is not zero. Up and down walk the rows in the order drawn. Enforced
by `the_arrows_walk_the_rows_in_the_order_drawn`. A click on a section's heading folds the
section to it, and a second click opens it. A folded section's rows are not walked. Enforced by
`a_section_s_heading_folds_its_sessions_away_and_opens_them_again`.

One action, `session.place`, puts a session at an index of a section. Drag and drop, the
palette and the row's menu all send it. Enforced by
`pausing_moves_the_session_and_keeps_it_on_disk` and
`a_move_within_its_section_reorders_it`.

**Drag and drop.** A row pressed and moved past the click's slop lifts on glass and follows the
pointer, held where it was gripped; its place stays ghosted, the section under the pointer lights, and a rule marks where
it lands. Released, it lands there. Enforced by `a_row_dropped_lower_in_its_section_lands_there`,
`a_row_dropped_under_the_paused_heading_is_paused` and
`a_press_that_does_not_move_places_nothing`.

**Palette and menu.** The palette offers `Pause session` or `Resume session`, and `Move to top`,
on the selected session. The row's right-click menu offers the same, and `Close`. No chord.
Enforced by `a_row_s_menu_pauses_its_session_and_a_paused_one_s_resumes_it`.

The sessions' header carries how many rows ask, `· 2 ask`, in the attention colour.

Each row:

1. **Kind tag** — three mono capitals in a fixed column: `TSK`, `EXP`, `MR` or `PR` by the
   review's forge. In the state's colour: peach while it asks, green while done and not seen,
   red on an error, blue while working, grey otherwise.
2. **Number, then title** — the number in the sub colour: `#148` for a GitHub issue or PR,
   `!88` for a GitLab MR, a Notion task's own id. An explorer has none.
3. **Agent status** — `working`, `asks to commit`, `done`, `idle`, `exited`, `error`. One
   line, truncated, never wrapped.
4. **Relative time, right-aligned** — how long the agent has waited, or since it finished.

```
 SESSIONS · 5 · 1 ASK                      +
 ───────────────────────────────────────────
 ▤  Board                                   2
 ───────────────────────────────────────────
 ACTIVE · 3
 TSK #148 Improve native theme            now
     working
     ├ TSK #150 Agent conversation   idle · 1h
     └ EXP flaky ci on macos         asks · 3m
 MR  !88 shop-deploy argo sync             2m
     asks to approve a push
 EXP kube scope picker                    40m
     idle
 PAUSED · 1
 TSK #141 Cluster writes                   2d
     idle
```

Nothing else: no CI, no MR state, no worktree count, no state dot. Rows run edge to edge
across the window with no rule between them, on square, see-through grounds: the selected one
held, the one under the pointer lighter. A row that asks lies on the attention colour's dimmed
tint, a done one not yet seen on the done colour's; the rest are bare. Headings keep the rows'
padding. Enforced by
`the_selected_row_stands_edge_to_edge_on_the_held_ground_and_the_hovered_one_lit`.

A row carries no button. An ask's text is peach, and a click on its row opens the session
with the review sheet on the first ask. Enforced by
`a_click_on_a_row_that_asks_opens_the_session_on_its_review`. With auto-approve on for the
session, nothing asks but a cluster write. A working status is blue and its glyph moves.

Opening a session starts its agent. Closing a row ends the agent and removes the row; the
session stays as it is on disk.

**Children.** A [child session](glossary.md#child-session) stands right under its parent on one
line, `├` and `└` as the feed's runs, its state and age inline. It has no section of its own:
it moves with its parent, is never dragged alone, and only the parent counts under a heading.
Enforced by `a_working_child_stands_under_its_idle_parent`.

**Feed.** In its own window under the sessions', the event log of the opened sessions, newest first, filterable to
the selected session: commits, pushes, MR events, notes, routines, cluster reads and Groove's own
errors and notes. Asks are never in the feed; they live on the row.
It reads as a log: one mono line an event, in three columns — how long ago, a short code for its
kind (`git`, `mr`, `note`, `rtn`, `kube`, `err`, `job`), then what happened, cut with `…` when
the window is narrow; across every session the text leads with the session's title. Colour marks
state only: a running job is blue and stands first, its age `··`; an error is red on its colour's
tint; every other code is faint. Back-to-back events of one kind from one session fold into one
line, `commit ×3 ▸`; a click opens it, held, its events hanging under it by `│` and `└`. A line
takes the user to its session.
Its top is the line the user drags; folded, it is its header alone, a bar tall, and the
sessions' window takes the column down to its header, nothing left to drag. Enforced by
`a_folded_feed_is_its_heading_alone_and_the_sessions_take_the_column`. Two tabs at its
heading's end, `this` and `all`, pick the session in hand or every session; they stand however
many sessions are open. Enforced by `the_scope_tabs_stand_whatever_number_of_sessions_is_open`.
Its header lights under the pointer as the sessions' does, square and edge to edge. Enforced by
`both_rail_headers_light_edge_to_edge_under_the_pointer`.
Enforced by `a_line_is_its_age_its_code_and_its_text_in_three_mono_columns`,
`back_to_back_events_of_one_kind_fold_into_one_row`,
`a_folded_run_opens_on_a_click_and_its_lines_lead_to_the_session`,
`the_scope_tabs_pick_the_session_in_hand_or_every_session`,
`an_open_run_s_line_is_held_and_its_lines_hang_bare_under_it`,
`the_feed_is_monochrome_but_for_its_errors` and
`an_error_lies_on_its_colour_s_tint`.

## Session — small scope

The rail at the left, then the session: a bar of its title and its tabs, the
[mode](glossary.md#mode) up in the middle under it, the sidebar at the right and the terminals' drawer at the middle's foot.

| Part | Holds |
|---|---|
| **Rail** | opened sessions, the feed |
| **Bar** | the session's title, the worktree it follows, the tabs |
| **Middle** | the mode up: the agent's conversation, the overview, the diff, a file or the cluster |
| **Sidebar** | the mode's own list, at the window's right edge; folds to its head |
| **Terminals** | a drawer at the middle's foot: their bar alone while folded, the grids open |

**The bar picks the mode.** Its title stands at the left, then the worktree every mode
follows, `repo · branch`, which a click opens the worktree picker on, as `alt+w` does; a long
title is cut before the worktree is. Its tabs stand at the right: `OVERVIEW`, `AGENT`, `DIFF`,
`FILES`, and `KUBERNETES` while the session holds a cluster. `alt+1` to `alt+5` pick them,
`alt+]` and `alt+[` step through them. The bar stands on nothing of its own. Enforced by
`the_bar_holds_the_title_and_the_tabs_and_the_agent_takes_the_middle`,
`a_tab_switches_the_middle_and_no_tab_says_it_comes_next`,
`the_bar_names_the_worktree_after_the_title_and_a_click_picks_another`,
`a_long_title_is_cut_so_the_worktree_stays_inside_the_header`,
`alt_brackets_step_through_the_modes_elsewhere` and
`alt_one_to_five_pick_the_tabs_and_the_sidebar_folds_to_its_head_and_drags`.

**The sidebar mirrors the rail.** Its window stands a gap inside the window's right edge, folds
to its head in the bar's row, the middle then taking the width, and its left border drags as the
rail's right one does. Every mode's sidebar scrolls under one scrollbar whose thumb drags, and
the arrows walk its rows as it draws them. `alt+o` hands the keys to the middle and `alt+e` to
the sidebar, opening it when folded; a second `alt+e` folds it. With both the rail's windows
folded the middle runs left across the rail's column, their heads standing over it as the
sidebar's does. Enforced by `the_sidebar_s_edge_drags_as_the_rail_s_does`,
`every_tab_s_sidebar_scrolls_under_one_scrollbar`,
`the_arrows_walk_the_sidebar_s_rows_as_it_draws_them`,
`alt_o_gives_the_keys_to_the_middle_and_alt_e_to_the_sidebar_then_folds_it`,
`with_both_rail_windows_folded_the_middle_takes_the_rail_s_column`,
`with_both_rail_windows_folded_the_agent_takes_the_whole_width` and
`the_rail_s_folded_heads_stand_over_the_middle_as_the_sidebar_s_does`.

**The terminals rise from the middle's foot.** Their drawer is nothing while the session has no
terminal; folded, it is its head across the middle's width as a rail window's is: `TERMINALS`
and their count, the fold, the tabs and the actions. Open, it rises to the height the user
dragged. The transcript stops at the middle's top; at its foot it scrolls on under the
terminals' head and the feed's; scrolled to either end, no row stands under them. Enforced by
`the_terminals_bar_shows_only_with_a_terminal_and_then_runs_the_middle_s_width`,
`plus_opens_the_section_with_a_terminal_its_grid_holds` and
`the_transcript_scrolls_on_under_the_bottom_bar_and_stops_at_the_top_one`.

**`AGENT` shows the conversation** with no line at its foot. Its sidebar holds the model, a
click on it opening the models alone, the usage gauges, and the agents it sent, each a click
away from its block in the conversation. Enforced by
`the_sidebar_holds_the_model_and_the_usage_and_the_foot_stays_bare`,
`the_sidebar_holds_three_gauges_and_its_model_row_opens_the_models` and
`the_agents_sent_stand_in_the_sidebar_and_no_dock_over_the_foot`.

**`OVERVIEW` shows the session's task, or the MR under review, on a card**, then each repo apart
on a card of its own: its name and remote, then a row for each worktree with its changes, MR and
pipeline; a click on a card's head folds it, and the column keeps the conversation's gutter. Its
sidebar holds the task's fields, the status in its colour, a missing one left out, or the
review's MR threads and pipeline, then the clusters, each context in its hue with its
namespaces, and a maintenance task's children. Enforced by
`the_overview_shows_each_repo_apart_with_its_worktree_in_a_row`,
`a_card_s_head_folds_the_card_on_the_overview`,
`the_overview_s_sidebar_heads_on_the_session_s_kind_and_folds`,
`a_property_the_task_lacks_is_left_out` and
`the_overview_names_each_context_once_with_its_namespaces_under_it`.

**`DIFF` shows one changed file at a time**, the first in the sidebar's order until a click
picks another. One band over its rows holds the file's name, folder and counts, then the views,
the modes and its read box. A thin map of that file stands beside the rows, all on the ground. A
commit shown says so in a band over the file, with what puts the working tree back. Its sidebar
heads on the whole change, its files and what it added and removed, then the strip of files,
commits and the MR's threads; the files stand as a tree whose directories fold, or as a list, a
right click switching the two. An edit's link in the conversation opens `DIFF` on that file at
its hunk, and a thread of the list opens it at its line. A cluster write takes the session to
`DIFF` until it is answered, then back. Enforced by
`the_middle_shows_the_first_file_alone_when_none_is_chosen`,
`a_click_on_a_file_in_the_sidebar_shows_it_in_the_middle`,
`the_sidebar_s_head_holds_the_whole_change_s_totals`,
`the_list_switches_between_tree_and_list_by_its_right_click`,
`a_directory_of_the_tree_folds_its_files`,
`the_wheel_over_the_middle_scrolls_the_code`,
`the_file_s_head_and_its_switches_share_one_band_over_its_rows`,
`a_commit_says_so_over_the_file_and_offers_the_working_tree_back`,
`a_press_low_on_the_sidebar_s_scrollbar_takes_the_list_to_its_end`,
`an_edit_s_file_opens_on_diff_shown_alone_at_its_hunk`,
`a_note_of_the_list_opens_the_line_it_stands_on` and
`a_cluster_write_takes_the_session_to_diff_and_back_once_answered`.

**`FILES` shows the open file on the ground under one band**: its path, then its unsaved dot,
counts, faults and language servers. Its sidebar heads on the repo, then holds `OPEN · n`, a row
each open file that a click shows and the middle button or its menu closes, then `WORKTREE`, the
worktree's tree with the changed files marked with their letter. A file opened from elsewhere
brings `FILES` up. Enforced by
`the_middle_shows_the_open_file_under_its_band_and_no_strip_of_tabs`,
`a_click_on_a_file_of_the_tree_opens_it_in_the_middle`,
`the_open_rows_list_the_open_files_and_a_click_shows_one`,
`the_middle_button_or_the_menu_closes_an_open_row`,
`a_directory_of_the_tree_opens_and_folds`,
`alt_o_on_files_gives_the_keys_to_the_code`,
`the_arrows_walk_the_open_rows_then_the_tree_as_drawn` and
`the_wheel_over_the_open_file_scrolls_it`.

**`KUBERNETES` shows the list or the object open on the ground under one band**, a thin rule
under it. The list's band names the kind and its count, then each context in its hue with its
namespaces, then the pin at the right. An object's band holds its name, its kind, context and
namespace, its status badges, then its views at the right; the logs' containers, run and range,
and an Application's filters, stand on a second row under it. Its sidebar heads on the contexts,
then holds `LISTS · n`, the pinned lists, `OPEN · n`, a row each open object that a click shows
and its cross, the middle button or its menu closes, then `KINDS`. The arrows walk the sidebar
as drawn, and an object opened from elsewhere brings `KUBERNETES` up. Enforced by
`the_list_band_names_the_kind_its_count_and_its_scope`,
`a_click_on_a_row_opens_the_object_under_open_and_the_middle_shows_it_with_its_views`,
`an_open_row_shows_its_object_and_its_cross_middle_button_or_menu_close_it`,
`the_arrows_walk_lists_open_and_kinds_as_drawn_and_left_folds_a_heading`,
`alt_o_on_kubernetes_gives_the_middle_the_keys_and_the_arrows_walk_its_rows`,
`the_logs_second_row_holds_the_containers_the_run_and_the_range_under_the_band` and
`the_wheel_over_the_middle_scrolls_the_list_and_over_the_sidebar_its_rows`.

On `DIFF` and `FILES` a thin rule stands under the band, and `ctrl+f` opens the search in a row
over it, as wide as it; on `KUBERNETES` it opens the list's find row there. Enforced by
`ctrl_f_opens_the_search_in_a_row_over_the_file_s_band_as_wide_as_it`.

**The agent pane draws the conversation.** Every agent runs headless: Claude Code's
stream-json on its pipes, no pty. A message, a skill, an interrupt and a model switch go over
that stream, and the transcript draws the blocks. It is a transcript, not a chat: no bubbles,
no avatars. The user's turn is a block across the column on a quiet ground under `YOU` and how
long ago; the agent's words are plain text, under `AGENT` once a turn. A kind is a word in
small capitals; the only marks are the fold chevron. Enforced by
`the_conversation_shows_in_the_agent_pane` and `the_agent_speaks_under_its_name_once_a_turn`.

**Steps fold into one block.** Reads, searches, commands and MCP calls back to back are
counted in words, `Read 3 · Searched 2 · Ran 1`, with the time they took and the last step
under. A thought says nothing and shows nothing. A click opens a row for each: its kind, its
target and its time, a command's last lines in a sunk box under it. While the agent works, a
line tinted in the accent stands over the block and says what runs and for how long. Enforced by `a_click_on_the_steps_opens_each_step_with_what_it_printed` and
`while_the_agent_works_past_its_last_record_a_working_line_says_so`.

**Edits are one block.** Files written back to back stand under `EDITED · N FILES` and their
sum; each file is a link to its hunk in the Diff with its `+N -N`. Enforced by
`edits_back_to_back_stand_in_one_block_each_file_a_link`,
`a_click_on_an_edit_s_file_opens_its_hunk_in_the_diff` and
`an_edit_in_another_worktree_selects_it_first`.

**What asks stands over the pane's foot.** A tool Claude asks leave for is glass ringed in the ask's
colour: `BASH ASKS LEAVE` and where, what it runs in a sunk box, *Refuse* on `ESC` and *Allow*
on `ENTER`. A question shows one at a time, its options on their digits with what each means,
and *Back* undoes the last pick. While the pane has the keys and the prompt is shut, Enter
allows, Escape refuses and a digit picks. With nothing asked, Escape interrupts the turn. Enforced by
`a_tool_s_ask_is_let_or_refused_from_its_card_and_from_an_empty_prompt`,
`a_question_s_options_are_picked_one_question_at_a_time_then_sent_together`,
`back_undoes_the_last_pick_of_a_question` and
`escape_on_an_empty_prompt_interrupts_the_agent_when_nothing_is_asked`.

**What goes wrong is red.** An API error, a reached limit or the agent ending by itself is a
red-tinted block: its title in small capitals, then what it printed. An interrupt is a rule
across the column, and nothing runs after it. A message typed while the agent works waits faint
and says so. Enforced by `what_went_wrong_shows_in_the_conversation` and
`an_interrupt_is_a_rule_across_the_column_and_nothing_runs_after_it`.

**An agent the agent sends is a block**: its type, its task, its state and its time, its steps
counted, its answer cut to two lines until a click. The sidebar holds one row for each agent
still working, and for each one done since the user last spoke; a click opens its block and
scrolls to it. Enforced by
`a_working_agent_keeps_a_row_in_the_sidebar_that_opens_its_card_far_up_the_conversation` and
`a_finished_agent_leaves_the_sidebar_once_the_user_speaks_again`.

**The prompt opens over everything.** At rest the pane has no prompt: the transcript runs to its
foot. `alt+a` opens the prompt, `alt+a` again closes it,
as do Escape and a send. A click or a chord elsewhere leaves it open, the keys going there, so
text can be copied into it, and the wheel scrolls what stands under the pointer; `alt+a` or a click on it takes the keys back. It is glass a layer
over the window, undimmed, as wide as the palette and centred over the window's foot, in the
focus ring while it has the keys. It holds
the commands `/` offers, the agent's then Groove's, each group under its head; then what is
typed, growing to eight rows and then scrolling; then its keys, with the model and the
context. Escape keeps the draft. Enter sends, shift+Enter breaks the line, Up in an empty
prompt brings back the user's last message. *Ask the agent*, in the menu of the diff's lines,
puts where they stand in the prompt and opens it. A drag over the transcript selects text and
copies it.
Enforced by `alt_a_opens_the_prompt_and_alt_a_again_closes_it`,
`escape_closes_the_prompt_and_keeps_its_draft`,
`enter_sends_the_message_and_closes_the_prompt`,
`the_prompt_stays_open_while_the_keys_go_elsewhere_and_alt_a_takes_them_back`,
`with_the_prompt_open_the_wheel_still_scrolls_the_transcript`,
`enter_sends_the_prompt_to_the_agent_and_shift_enter_breaks_its_line`,
`up_in_an_empty_prompt_steps_back_through_what_was_sent_and_down_returns`,
`picking_a_model_switches_the_agent_to_it`,
`a_slash_offers_the_agent_s_commands_and_tab_takes_the_chosen_one`,
`the_slash_offers_the_agent_s_commands_then_groove_s_under_their_heads`,
`asking_the_agent_about_lines_puts_where_they_stand_in_its_prompt`,
`a_long_line_of_code_the_agent_shows_wraps_instead_of_running_out_of_sight`,
`a_drag_over_the_conversation_copies_what_it_holds`,
`a_double_click_takes_a_word_and_a_triple_click_its_line`,
`ctrl_and_a_click_follow_a_link_or_open_a_file_at_its_line`,
`the_prompt_keeps_its_spaces_and_selects_and_copies_as_a_note_does`,
`a_double_click_in_the_prompt_takes_its_word` and
`the_prompt_follows_its_caret_past_its_tallest_and_the_wheel_scrolls_it`.

The rail and the sidebar each have a width the user drags, kept between runs; the middle takes
what is left and never less than its minimum. A boundary moves the two parts it stands between
and nothing else. Enforced by `a_press_on_a_boundary_takes_hold_of_it`,
`a_drag_never_starves_a_column` and `the_sidebar_edge_leaves_the_middle_its_minimum`.

### Workspace

**Actions are palette rows.** Finishing the task and deleting the session here are palette
rows. A button that is one icon alone is a circle. Enforced by
`a_task_or_a_review_is_deleted_here_from_the_palette_and_an_explorer_is_not` and
`an_icon_alone_is_a_circle_under_the_pointer`.

**The scope only tells.** `alt+c` opens the context panel and `alt+n` the namespace panel,
from any mode, where the palette stands, with a search and the first line stood on. The context
panel holds the session's contexts, a check to show one and a × to detach it, then what it could
attach; a pick there attaches the whole cluster. The namespace panel lists `context/namespace`
for every context held, `context/*` for the whole cluster, the best fuzzy match first: space
shows one more, Enter shows one alone and gives the list the keys. Either choice brings
`KUBERNETES` up. Enforced by
`the_scope_only_tells_and_alt_n_opens_the_panel_from_any_mode`. A title or a label too long
for its line is cut with an ellipsis; no line wraps; an object's status badges keep their room
before the cut, and a badge with no room left is dropped.

**One code surface.** The Files mode's editor and the Diff's two views are one surface,
which the gutter holds together.

| Mode | Shows | Gutter |
|---|---|---|
| files | the file as it is now | a mark on every line the change touched |
| inline | the change as one column | the old and the new line number |
| split | the old beside the new | one number per side |

An added or a removed row wears one tint across its gutter and its text; its numbers stay
quiet. Enforced by
`a_changed_row_wears_one_tint_across_its_gutter_and_its_numbers_stay_quiet`.
In split, a side with no line is hatched, gutter included.

Over the diff, the band names the focused file, then the inline and split switch and the base
and working modes as rounded tabs. Over a file, the band names the file from the left, then what
stands beside it: unsaved, changed, faults, servers. Enforced by
`a_file_s_band_reads_from_the_left_its_counts_right_after_its_path` and
`the_file_s_band_counts_its_errors_and_names_its_server`. No rule stands between the gutter and
the code; a run of unchanged lines stands between two rules, its arrows icon buttons. Enforced by
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

**Search.** In the surface, `ctrl+f` opens a row over the band with the count at its end; it
finds in the file shown. Enter and shift+enter step to the next and the previous match, esc ends
it and leaves the caret on the match. `alt+f` opens the palette on the worktree's files.
Enforced by `enter_and_shift_enter_step_the_matches_and_wrap` and
`alt_f_opens_the_palette_on_the_worktree_s_files`.

**Finding your way in a big change.** Three answers, and none of them is a bigger tree:

- **One file at a time.** The Diff shows the focused file alone; the sidebar's tree, or its
  flat list, picks the next.
- **Where am I.** In the file view, the scopes the visible rows sit in pin at the top, stacked
  as they nest, read from the syntax tree.
- **What is left.** One column down the edge holds the focused file: its band, its hunks as
  marks, the viewport as a lens that drags. A file marked read dims. A file is marked read by
  the user, never by scrolling past it. The marks belong to the session and outlive the window.
  Enforced by `the_map_holds_the_focused_file_s_band_and_a_lens_over_the_rows`.

**Manual section.** Where the user acts by hand, as the agent pane is where the agent acts.
Terminals for the session, opened in its own directory, splittable, resizable, collapsible;
its bar alone when collapsed.

**Git.** The palette's Git group acts on the selected worktree: push, pull and the MR's
own actions. No box in the window commits: the agent or a terminal
does.

**One deep ground, panels on it.** The deepest ground the theme has holds the session's
middle, the terminal widget, the board and Settings. The panels that stand beside each other on
it — the rail's two windows, the sidebar, the terminals' drawer, Settings' sections — are
opaque: the theme's base, edged by a hairline.
Nothing passes under a panel, so glass would only show the same ground through it.
The defense themes, dark and light, lay a hex lattice on that deepest ground: the faint edges of
hexes 18 pixels from centre to corner, pointy at the top, counted from the window's corner, under
everything — the panels cover it, and it runs faintly behind the agent's conversation. No other theme
lays one. Enforced by `the_defense_themes_lay_a_hex_lattice_on_the_deep_ground_and_the_others_none`.
While the session's agent works, signals fire across the session's lattice: each starts at a corner
anywhere on the ground, runs the hexes' edges down, fires the corner it reaches and branches on, the edges it
crossed glowing behind it and fading. When the agent stops, so do they. Enforced by
`signals_fire_across_the_lattice_only_while_the_session_s_agent_works`.
Each panel is headed by its band — a step from its base toward the theme's first surface —
ruled off under it by a hairline; a panel folded to its header is that band alone, laid before
what stands on it. Enforced by `every_open_panel_is_headed_by_its_band_ruled_off_under_it`. A
note floats over the rows it takes as one pane of glass, its replies and its buttons ruled off
on it; a reply being written stands in that pane, its field a rounded field. Enforced by
`a_read_note_floats_over_its_rows_as_one_pane_of_glass` and
`a_reply_being_written_stands_on_glass_as_a_read_note_does`.
The sidebar's rows are lit and held across it whole, and the ring goes around its window when
it has the keys. The diff's map runs the column's whole height, square-cornered, its lens too,
and a changed row says so by its ground alone, its numbers left quiet.

**Glass only where it floats.** What stands over something is glass: the palette, every
menu, the cluster scope panel, the agent's prompt and what it asks, the language servers' popup,
the pinned scopes and the notes: the work below shows through them, blurred
under the theme's tint; nothing bends, magnifies or lights them. The completion
list stays solid. A row on glass takes a rounded, see-through ground under the pointer and when selected; a row of the rail, a sidebar or a table takes its square, edge to edge. A button on
glass is a pill of glass, lit whole under the pointer. Enforced by
`the_rail_stands_as_two_opaque_windows_inside_its_place`,
`a_flat_button_lights_on_a_rounded_ground_and_a_worktree_row_on_a_square_one`,
`the_palette_is_glass`,
`a_menu_is_glass`,
`the_panel_stands_on_glass`,
`the_hover_popup_is_glass`,
`the_completion_list_stays_solid` and
`a_palette_row_under_the_pointer_takes_a_square_see_through_ground`.

**The code's popups are cards.** The language servers' popup is ringed and tinted in the worst
diagnostic's colour, muted without one. Each diagnostic stands under a head in small capitals,
its kind and line in its colour, `WARNING · LINE 31`, then its message; the hover stands under
its own head. The completion list keeps its rows under a tinted head that counts them. Enforced
by `a_fault_s_popup_heads_on_its_kind_and_line` and
`a_diagnostic_heads_on_its_kind_and_line_in_its_colour_then_its_title_details_and_source`.

**Three grounds a row can take, and they never share a value.** Under the pointer is the
quietest, a selected row is stronger, and what a click acts on is stronger again. Where the
caret is, two rules stand instead of a ground: a place is not a state. The row a list stands
on — the open file, the shown commit, the kind listed — has the two rules over the held
ground, the accent dimmed into it. The rail is the exception: see below.

**The palette stands as the rest does.** Its glass is ringed in the accent, for it has the
keys; the query stands on its band, ruled off across the panel's width, with no line of its own; a group's heading carries its
count, `GIT · 2`; its rows are lit and held square, edge to edge; and a chord stands as its keys,
each in its own hairline box, `ALT` `⇧` `B`, shift as its sign; the last row meets the panel's
bottom edge. The context and namespace panels stand the same, their headings `IN THIS SESSION · 1`.
Enforced by `a_chord_stands_as_its_keys_each_in_a_hairline_box`,
`a_palette_row_under_the_pointer_takes_a_square_see_through_ground`,
`the_last_row_selected_reaches_the_panels_bottom` and
`the_scope_panel_stands_as_the_palette_does_on_its_band_with_headings_in_capitals`.

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
match in the file name first, and opens the one picked in `FILES`.
Enforced by `a_hash_searches_the_worktree_files_and_opens_the_one_picked`.

**Git and the MR go through the palette.** Push, pull, and
create, update, comment on, approve or close the MR are palette rows of the selected
worktree. No box in the window holds them.

**A question takes the place of what asked it.** Closing an open file with unsaved edits,
deleting a path or a write on a resource's row turns that line into the question, square across its
whole width. Its two answers follow the words, no modal, and nothing destructive within a
click of something ordinary.

### Kubernetes mode

- **List.** The list, under its band of its kind and its count, over the rows of every context
  and namespace in scope: the server's Table columns `kubectl get` shows, behind a cluster
  column and a namespace column where those tell rows apart; a status and a ready count in
  the colour of how the object stands. A time cell, the age or a last schedule, counts up
  live; the age from the object's creation. A header sorts by its column, numbers and ages
  by value, and a second click turns the order round; its edge drags the column's width,
  kept for the kind. `ctrl+f` opens a find bar over the rows: fuzzy name words —
  `bil-eu-st` keeps `billing-eu-staging` — and `label=value` words the server filters
  on. `alt+c` opens the context panel, `alt+n` the namespace panel, `alt+k` the kind palette. The table
  is a grid: its header in capitals on a band, a hairline between the columns and under each
  row; a row under the pointer or selected is tinted square, edge to edge. Enforced by
  `a_gridded_table_bands_its_header_and_rules_each_column_and_row`. The find bar is a rounded
  field.
- **Object.** A row opens its object under `OPEN` in the sidebar, `describe · yaml`, and
  `logs` for a pod. The object is read, with its events and its owners by name, and a pod's
  usage every 15 s, while it shows. As YAML it is the file editor's own view of the object,
  read-only.
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
  sunk card as the overview's do, its heading on the card's band, ruled off under it and lit
  whole with its caret under the pointer; a folded one is its band alone. Every entry stands on
  a line of its own, a condition included, and the containers and the relations are the list's
  grid. Enforced by
  `a_section_stands_on_a_sunk_card_headed_by_its_band_and_its_conditions_a_line_each`. A copy under the pointer lights on a square glass
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
  resource's row opens its object there. A row's right-click offers the skills of its kind, as
  a list row's does. The find key opens a find bar under its filters, as over the list: its
  words narrow the rows by name, and the filters count what they hold, the bar aside. A git source's address, ctrl+clicked, opens the
  files on its repo, selected in the session or added from the pool, cut from the branch
  the source follows when it is one; its synced commit, ctrl+clicked, shows in the diff as
  a commit of the list does, fetched first when the clone lacks it. Its `resources` view
  filters by what needs attention (out of sync, to prune, or failed in the last sync), out
  of sync, syncing or all, each in its colour with its count; it opens on what needs
  attention, or on the sync while one runs. Its filters are glass tabs. Both it and syncing
  are the list's table, its headers sorting each group's rows and its edges dragging widths
  kept for both; a group's heading stands on a band, ruled off as each row is. Resources stand by
  kind, the kinds needing attention first; under all, a kind with nothing to see stands
  folded. Syncing shows the sync's line on its colour, over a square bar that
  fills resource by resource, then the PreSync hooks, each wave in order with its Sync
  hooks on a badge, and the hooks after, each step applied, applying, waiting or failed; a
  stage done stands folded.
- **Pins.** **A pinned list keeps its kind and its scope, and belongs to the session.** The
  list's pin keeps it under `LISTS` in the sidebar: the kind, and the contexts and
  namespaces it reads as the list shows them. A click on it brings that list back; the pin
  again or its cross lets it go. A list not pinned stands in the middle alone, and picking a
  kind replaces it. Pins outlive the window. Enforced by
  `pin_keeps_the_list_under_lists_a_click_brings_it_back_and_pin_or_its_cross_lets_it_go`
  and `a_pinned_list_comes_back_with_its_session_and_an_unpinned_one_does_not`.
- **Kinds.** Under `KINDS`, the kinds the contexts serve, indented under headings that fold:
  workloads, network, config, storage, RBAC and other for Kubernetes' own, one a CRD group by
  its name, then cluster for the built-in kinds not namespaced. No counts, no health marks. A
  heading or a kind under the pointer lights, and the kind listed stands on a square held
  ground. Enforced by `a_crd_stands_under_its_own_group_and_the_rbac_kinds_under_theirs`,
  `the_kind_listed_stands_on_a_square_held_ground` and
  `the_arrows_scroll_the_kinds_to_keep_the_line_stood_on_in_sight`.

### Review sheet

An ask opens a sheet over the middle's top, as tall as its text. It stands as the agent's own asks do: glass ringed in the
ask's colour, a tinted head saying what it asks in small capitals with the session, the repo and
the branch, the write in mono, then the answers. The rest of the middle stays visible. The sheet shows only an ask of the session
selected: it opens on arriving at a session that asks, from its rail row or with `a` and `v`,
never by itself while the user stays on a session, and never for another session's ask. One frame for every op: the repo and the branch, then the text of the
write. No diff and no file list. A push goes to the worktree's own branch and nowhere else.

| Op | Text shown |
|---|---|
| commit | the message: title and body |
| push | each commit's title and body |
| MR create, update, close | the MR's title and body |
| task property | the task's title, the property, before → after |
| task body | the task's title and the body |
| task finish | the task's title and the worktrees torn down |

**Approve** in peach on `ENTER`, **Refuse** on `ESC`. No editing in the sheet. Enforced by
`escape_refuses_the_write_on_the_sheet_and_enter_approves_it`,
`arriving_on_a_session_that_asks_opens_its_sheet_and_another_s_never_shows` and
`the_sheet_floats_over_the_middle`.

### Cluster write

**A session waiting on a cluster write answers it first, on its Diff.** The session stands on
`DIFF` whatever tab is asked. A band on a dim attention
tint holds what the write asks, in capitals, then the object, with **Refuse** and
**Approve** at its right once the change is shown, bordered words in capitals, Approve
edged in peach. Under it, the
diff itself, read-only, over the object's YAML before → after: the change stands in place
of the worktree's until the write is decided, then the worktree's is read again. No sheet
stands over it: Esc, Enter, the `a` key and the palette decide nothing, and closing,
deleting or reloading the session is refused until it is answered. Another session can be
selected meanwhile; back on this one, the change shows again. Once the write is answered,
the session goes back to the mode it stood on. Enforced by
`the_diff_holds_the_ask_then_the_change_whatever_tab_was_asked`,
`once_the_write_is_answered_the_session_goes_back_to_the_mode_it_stood_on`,
`a_on_the_rail_neither_approves_nor_opens_a_sheet_over_the_ask`,
`only_the_band_s_approve_button_lets_a_cluster_write_through`,
`a_write_is_answered_only_once_its_change_is_shown`,
`once_one_write_is_answered_the_next_one_s_change_is_shown`,
`a_session_waiting_on_a_cluster_write_is_not_closed_before_it_is_answered` and
`a_review_takes_the_change_s_place_until_its_own_write_is_decided`.

**An object scales, restarts and goes from its row.** A list row's right-click offers
**scale** on a Deployment or a StatefulSet, **restart** on those and a DaemonSet, and
**delete** on every object, above the skills of its kind. Picking one turns the row into the
question. A scale shows the count the row wants between **−** and **+**, no **−** at none,
and sends the count set to the `scale` subresource. A restart stamps the pod template, as
`kubectl rollout restart` does. A delete takes the object's dependents after it. **keep**
sends nothing; the timeline says what landed. An open object's row offers its own menu, not
the skills. Enforced by
`a_deployment_s_row_asks_its_restart_in_its_place_and_restart_sends_it`,
`keep_leaves_the_deployment_as_it_is`,
`a_scale_starts_from_the_count_the_row_wants_and_steps_before_it_is_sent`,
`a_scale_to_none_offers_no_step_down`,
`a_pod_s_row_offers_its_shell_then_its_delete_asked_in_its_place`,
`an_open_object_s_row_opens_its_own_menu_and_not_the_skills`,
`a_restart_stamps_the_pod_template_of_a_workload_and_refuses_a_pod`,
`a_scale_patches_the_scale_subresource_of_a_deployment_and_refuses_a_daemonset` and
`a_dry_run_delete_says_so_in_its_body_on_the_version_seen`.

**A pod's row opens a shell in it.** Its right-click offers **shell** first, one per
container once the pod's tab has read several, kubectl's own pick otherwise. It opens the
terminal drawer on a new tab named after the pod and its container, the keys going to it,
running `kubectl exec` with bash where the container has it, sh otherwise. Nothing asks
first, and a context set read-only refuses it; a shell already open stays. Enforced by
`a_pod_s_shell_opens_the_drawer_with_the_keys_going_to_it`,
`a_pod_s_shell_opens_in_a_tab_named_after_its_container`,
`a_read_only_context_refuses_a_pod_s_shell` and
`a_shell_in_a_pod_takes_the_keys_on_a_terminal`.

**An object's YAML is edited from its bar.** A bar between the object's band and its YAML
starts with **EDIT**: until it is pressed the YAML is read-only and no key changes it.
**EDIT**, where the session may write and the context is not read-only, starts a draft from
the YAML as it stands; every key, a paste included, then edits it. **EDIT** gives its place
to **APPLY** and **DISCARD**, which throws the draft away; all are bordered words in capitals, **APPLY** and **EDIT** in peach as **Approve**
is. The draft stands while the object moves on, the bar saying so. **APPLY** reads the
draft back as a merge patch, refused when it does not read, changes nothing or touches what
the server writes, and tries it on the version the draft started from: an object that moved
since is refused there. The edit then waits as a cluster write does, on the Diff under
**APPLY EDIT TO A CLUSTER OBJECT**; **Approve** makes it on that version and ends the draft,
**Refuse** keeps it. Enforced by
`an_edit_of_the_yaml_is_reviewed_then_lands_on_its_version_and_ends_the_draft`,
`an_edit_of_what_the_server_writes_is_refused_and_says_which`,
`the_yaml_stands_read_only_until_edit_is_pressed`,
`a_draft_takes_every_edit_and_stands_while_the_object_moves_on` and
`a_read_only_context_keeps_an_object_s_yaml_read_only`.

## Settings

Config's surface. Takes the whole window, the rail included; Esc or *back* returns to where
the user was. Reached from its chord, `alt+,`, and the palette. A section list on the left,
on a floating pane of glass as the rail's — Setup, Providers, Clusters, Alerts, Agent,
Appearance, Preferences, Keymap — under the title, *back* and a search field: typing filters every
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
| Alerts | incident.io: the `inc` login as the last alerts read found it, with *log in*, which runs `inc auth login` on Settings' terminal · the poll interval |
| Agent | auto-approve default · the shared repo, by URL and branch · the skills — core, the user's own, shared — each with its toggle · the routines — pause all, how many run at once, each routine and its triggers |
| Appearance | theme — Latte, Frappé, Macchiato, Mocha, Defense (teal-black and steel, cyan for what is live and for focus, amber for what asks, coral for what fails; its terminals in the same colours), Defense light (the same by day: pale steel, deep cyan, burnt amber, red) · UI font: Inter · mono font: IBM Plex Mono, JetBrains Mono, Lilex · three sizes: interface, editor, terminal · reduce motion |
| Language servers | each server the config names: its switch, the languages it serves, its program and what the check found it to be |
| Preferences | attention thresholds — review waiting, due soon and approved unmerged in days, a failed run in minutes · the forge poll's interval and its stale threshold |
| Keymap | every action with its chords; a chord is rebound by pressing it |

Every change saves to the config file on the spot. No save button.
