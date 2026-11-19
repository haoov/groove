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
- **Focus is a ring**: the region that has the keys — the sessions' window, the strip, the
  sidebar, the terminals — stands in a thin ring in the accent, read as half its strength on
  either theme. The agent's screen has no edge to ring: its cursor turns a solid block in the
  accent while it has the keys. A terminal shows them by a solid cursor too. Enforced by
  `the_ring_stands_around_the_region_that_has_the_keys_and_nowhere_else`,
  `the_agent_has_no_ring_and_its_caret_takes_the_accent` and
  `a_selection_stays_neutral_in_the_region_that_has_the_keys`.
- **A region is one chord away.** `alt+r`, `alt+a`, `alt+o` and `alt+e` hand the keys to the
  rail, the agent, the workspace and the sidebar; `alt+i` hands them to a maintenance task's
  alerts; the terminals keep their own chord. The
  workspace, the sidebar and the alerts unfold first, and a press while they hold the keys folds them
  away. `ctrl+alt` with `h`, `j`, `k` or `l` moves the boundary of the region that holds the keys
  that way.
  Enforced by `alt_and_a_region_s_letter_hand_it_the_keys`,
  `a_second_press_folds_the_sidebar_away`,
  `alt_o_opens_the_workspace_then_gives_it_the_keys_then_folds_it` and
  `ctrl_alt_and_a_direction_move_the_focused_pane_s_boundary_that_way` and
  `alt_i_unfolds_the_alerts_gives_them_the_keys_then_folds_them`.
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

**No header.** The columns' headings stand at the top; the board has no filter and no
button. An explorer starts from the rail or the palette. The columns keep the agent's room:
clear of a bar's height above and below, and across the rail's column once both its windows
are folded. Enforced by `the_board_opens_straight_on_its_columns_headings` and
`the_board_keeps_the_agent_s_room_and_with_the_rail_folded_takes_its_width`.

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
agent, as the workspace's chord does. Enforced by
`a_click_on_the_sessions_header_folds_the_window_to_it_and_the_feed_takes_the_column`,
`both_folded_the_rail_is_its_two_headers`,
`the_rail_s_chord_opens_the_folded_sessions_and_hands_them_the_keys` and
`the_rail_s_chord_on_the_rail_folds_the_sessions_and_hands_the_keys_to_the_agent`.
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

1. **Type icon** — task, review, explorer, routine, in the state's colour: peach while it asks,
   green while done and not seen, red on an error, blue while working, grey otherwise.
2. **Title.**
3. **Agent status** — `working`, `asks to commit`, `done`, `idle`, `exited`, `error`. One
   line, truncated, never wrapped.
4. **Relative time, right-aligned** — how long the agent has waited, or since it finished.

```
 SESSIONS · 4                              +
 ───────────────────────────────────────────
 ▤  Board                                   2
 ───────────────────────────────────────────
 NEEDS YOU · 1
 ⚑  TASK-51  auth refresh                  2m
    asks to commit
 WORKING · 1
 ◎  review-3  shop argo                   now
    working
 IDLE · 2
 ⚑  fix/keys-50                            6m
    done
 ⚑  TASK-48  finops export                 3h
    idle
```

Nothing else: no CI, no MR state, no worktree count, no state dot. Rows run edge to edge
across the window with no rule between them, on square, see-through grounds: the selected one
held, the one under the pointer lighter, the rest bare; no row is tinted. Headings keep the
rows' padding. A row that asks shows it in its icon and its peach state line. Enforced by
`the_selected_row_stands_edge_to_edge_on_the_held_ground_and_the_hovered_one_lit` and
`a_row_that_asks_shows_its_icon_in_the_attention_colour_on_no_tint`.

A row carries no button. An ask's text is peach, and a click on its row opens the session
with the review sheet on the first ask. Enforced by
`a_click_on_a_row_that_asks_opens_the_session_on_its_review`. With auto-approve on for the
session, nothing asks but a cluster write. A working status is blue and its glyph moves.

Opening a session starts its agent. Closing a row ends the agent and removes the row; the
session stays as it is on disk.

**Children.** A [child session](glossary.md#child-session) stands right under its parent,
indented, whatever its own state; only the parent counts under a state's heading. Enforced by
`a_working_child_stands_under_its_idle_parent`.

**Routines.** A standalone routine's session stands under its own heading, `Routines · N`,
folded by default, below the other sessions. Its row offers a round glass play button, which
runs the routine now, while no run is on it. Selected, it shows its agent pane alone, full width. It never
appears on the board.

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

The rail, then one ground for the agent with the workspace's strip standing on it.

| Part | Holds |
|---|---|
| **Rail** | opened sessions, the feed |
| **Agent pane** | the agent's pty, a bar's room left over it and under it |
| **Strip** | header · info line · the selected tab · the sidebar |
| **Sidebar** | inside the strip: contextual list for the selected tab; folds away |
| **Terminals** | a drawer under the strip: a capsule while folded, the grids open |
| **Alerts** | a maintenance task's firing alerts, over the agent: their header alone while folded |

**The strip folds to its header.** A click on the open tab folds it: the header alone stays
at the top right, a capsule as narrow as its title and tabs, and the agent takes the
ground's whole width. A click on any tab opens it again on that tab. The agent's terminal
takes its new size at once. Folding the terminals leaves a folded strip folded. Enforced by
`a_click_on_the_open_tab_folds_the_strip_and_widens_the_agent`,
`any_tab_opens_the_folded_strip_on_itself`,
`the_folded_strip_is_a_capsule_as_narrow_as_its_title_and_tabs` and
`folding_the_drawer_leaves_the_strip_folded`.
The agent's screen leaves a bar's room over it, where the folded header's capsule stands, and one
under it. The header, open or folded, and the terminals' drawer stand a layer over the screen,
never under its text. Enforced by
`the_folded_header_stands_a_layer_over_the_agent_s_screen`. With both the rail's windows folded, the
screen runs left across the rail's column to the window's edge, its two headers in the rooms over
and under it; the wheel there scrolls the agent and the rail's edge is not dragged. Enforced by
`with_both_rail_windows_folded_the_agent_takes_the_whole_width`.

**A maintenance task's alerts stand over the agent.** Open, they take the bar's free room over
the agent, down to the foot the user drags, the agent under them. Folded, they are their header alone in
the bar over the agent; with everything folded, that header stands after the sessions' header
and before the strip's capsule. The header counts the alerts by rank. They list on the kit's grid, one
line each: the priority in its colour, the alert, its environment, its cluster in its hue, its
namespace, its age, its child's state, then an Investigate or Open button. A header orders the
rows and its edge drags the column's width. A click on a row selects it and gives the alerts the
keys: the arrows step through the rows and Enter investigates; only the button or Enter spawns or
opens a child. A resolved alert stays,
greyed, while its child is on the rail. Enforced by
`open_the_alerts_stand_over_the_agent_which_keeps_the_rest_of_its_height`,
`folded_the_alerts_are_their_header_in_the_bar_over_the_agent`,
`with_everything_folded_the_alerts_take_the_bar_between_the_sessions_header_and_the_strip`,
`the_alerts_count_by_rank_and_list_on_one_line_of_a_grid_with_a_button_to_investigate`,
`an_alerts_column_drags_its_width_and_its_header_orders_the_rows`,
`a_row_takes_the_keys_and_only_its_button_or_enter_investigates`,
`the_alerts_foot_is_dragged_and_the_agent_follows` and `a_session_not_marked_has_no_alerts`.

**The terminals rise from the foot.** Their drawer is an opaque panel under the strip.
Folded, it is a capsule at the bottom right, as narrow as its bar. Open, it rises to the
height the user dragged and the strip ends a gap above it; with the strip folded it rises to
the strip's header, its column's edge still dragged, and the agent gives it the column until it
folds again. A panel open beside a folded one takes the room it leaves.
Enforced by `the_folded_drawer_is_a_capsule_as_narrow_as_its_bar`,
`an_open_drawer_rises_its_height_and_the_strip_ends_above_it` and
`an_open_drawer_rises_to_the_folded_strip_s_header_beside_the_agent`.

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
selects one. On the Kubernetes tab that line holds the tab's scope in the worktree's place,
led by a cluster mark: the contexts shown, a lone one in its hue, then the namespaces
(`on_the_kubernetes_tab_the_info_line_holds_the_scope_in_the_worktree_s_place`). It only tells: `alt+c`
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
| overview | sections, each a card set into the panel, a step below its base, inset from its sides and headed by its band — the section's name and, where it counts, how many — its rows ruled apart: the task's properties in columns, each name over its value — status, priority, dates, the hours logged — a missing one left out, sized to what they hold; the repos, each with its worktrees as rows read from the left — the branch, git's counts, then the MR's number, verdict, CI status and notes, zero counts and an absent MR omitted; the clusters it holds, each context in its hue with its namespaces after it; the body as text. Enforced by `each_section_stands_in_its_own_card_inset_from_the_panel` | folded |
| diff | the whole change as one stream, inline or split, notes inline, editable on its new side | one search field, always there, a hairline box with no ground, for a path or for text, a click on the other term's name switching it; then a strip that picks the list, its rows lit and held on square grounds edge to edge as the rail's are (enforced by `one_search_field_always_stands_over_the_strip_and_switches_its_term`): files — the files that changed, a click scrolls the stream to one, with stage, unstage and discard by right click; commits — the branch's own log, a commit opens its change read-only; notes — the session's annotations and threads, open, resolved or all, under the file each stands on and the MR's own last, two lines a note; a note scrolls to its line, a resolved one opened |
| files | the open files, one tab each with a dot while it owes the disk; the active one on the code surface, marked where the change touched it | the search bar, then a strip that picks the list: notes — the diff's own list, a note opening its file at its line; files — the whole worktree as a tree with the path operations by right click; a click opens a file in the preview tab, in italic, which the next click replaces; a double click or the first edit keeps it. A found line opens at the line. A tab's right click offers close, close others and close all, which leave an unsaved file open; a middle click closes it |
| kubernetes | the list tab, titled by its kind and its count, and a tab for each object opened from it; [Kubernetes tab](#kubernetes-tab) draws both | the kinds the contexts serve, under headings that fold |

**One code surface.** The Files tab's editor and the Diff tab's two views are one surface,
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

Over the diff, a band holds the whole change from the left — how many files, what it added
and removed — then the inline and split switch and the base and working modes as rounded tabs. Over a file, the band names the file from the left, then what stands beside it: unsaved,
changed, faults, servers (enforced by `a_file_s_band_reads_from_the_left_its_counts_right_after_its_path`). No rule stands between the gutter and the code; a run of unchanged lines stands between two
rules, its arrows icon buttons. A file's head is a square band across the surface whole, pinned or not. Enforced by
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

**Git.** The palette's Git group acts on the selected worktree: push, pull and the MR's
own actions. No box in the window commits: the agent or a terminal
does.

**One deep ground, panels on it.** The deepest ground the theme has holds the session
under the agent and the strip, the terminal widget, the board and Settings. The panels that
stand beside each other on it — the rail's two windows, the strip and its folded capsule, the
terminals' drawer, Settings' sections — are opaque: the theme's base, edged by a hairline.
Nothing passes under a panel, so glass would only show the same ground through it.
The defense themes, dark and light, lay a hex lattice on that deepest ground: the faint edges of
hexes 18 pixels from centre to corner, pointy at the top, counted from the window's corner, under
everything — the panels cover it, and it runs faintly behind the agent's screen. No other theme
lays one. Enforced by `the_defense_themes_lay_a_hex_lattice_on_the_deep_ground_and_the_others_none`.
While the session's agent works, signals fire across the session's lattice: each starts at a corner
anywhere on the ground, runs the hexes' edges down, fires the corner it reaches and branches on, the edges it
crossed glowing behind it and fading. When the agent stops, so do they. Enforced by
`signals_fire_across_the_lattice_only_while_the_session_s_agent_works`.
Each panel is headed by its band — a step from its base toward the theme's first surface —
ruled off under it by a hairline; a panel folded to its header is that band alone, laid before
what stands on it. Enforced by `every_open_panel_is_headed_by_its_band_ruled_off_under_it` and
`a_folded_capsule_is_its_band_and_what_it_holds_stands_on_it`. A note floats over the rows it takes as one pane of glass, on the layer
the sticky file head uses, its replies and its buttons ruled off on it; a note being written
does the same, its field a rounded field. Enforced by
`a_read_note_floats_over_its_rows_as_one_pane_of_glass` and
`a_note_being_written_stands_on_glass_as_a_read_one_does`.
The sidebar is a box a step below its panel's base, from the info line's rule to the panel's
edges, its rows lit and held across it whole; its own border is its separator, and the ring goes
around that box when the sidebar has the keys. The diff's map runs the column's whole height,
square-cornered, its lens too, and a changed row says so by its ground alone, its
numbers left quiet.

**Glass only where it floats.** What stands over something is glass: the palette, every
menu, the cluster scope panel, and the language servers' popup
the diff's sticky file heads and its notes: the work below shows through them, blurred
under the theme's tint; nothing bends, magnifies or lights them. The completion
list stays solid. A row on glass takes a rounded, see-through ground under the pointer and when selected; a row of the rail, a sidebar or a table takes its square, edge to edge. A button on
glass is a pill of glass, lit whole under the pointer. Enforced by
`the_rail_stands_as_two_opaque_windows_inside_its_place`,
`the_strip_stands_as_an_opaque_panel_on_the_agent_s_ground`,
`the_palette_is_glass`,
`a_menu_is_glass`,
`the_panel_stands_on_glass`,
`the_hover_popup_is_glass`,
`the_completion_list_stays_solid` and
`a_palette_row_under_the_pointer_takes_a_square_see_through_ground`.

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
match in the file name first, and opens the one picked on the Files tab.
Enforced by `a_hash_searches_the_worktree_files_and_opens_the_one_picked`.

**Git and the MR go through the palette.** Push, pull, and
create, update, comment on, approve or close the MR are palette rows of the selected
worktree. No box in the window holds them.

**A row offers, it never surprises.** Pointing at a file shows one word at its end — stage,
or unstage — where its counts were. Everything else is behind the right button.

**A question takes the place of what asked it.** Closing a tab with unsaved edits, deleting
a path or a write on a resource's row turns that line into the question, square across its
whole width. Its two answers follow the words, no modal, and nothing destructive within a
click of something ordinary.

### Kubernetes tab

- **List.** The list tab, titled by its kind and its count, over the rows of every context
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
  `a_gridded_table_bands_its_header_and_rules_each_column_and_row`. The list's tab
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
  resource's row opens its tab there. A row's right-click offers the skills of its kind, as
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

**Approve** in peach, **Refuse**. No editing in the sheet. Esc closes it without deciding.

### Cluster write

**A session waiting on a cluster write answers it first, in its Diff tab.** The workspace
stands on the Diff tab, ringed in peach, whatever tab is asked. A band on a dim attention
tint holds what the write asks, in capitals, then the object, with **Refuse** and
**Approve** at its right, bordered words in capitals, Approve edged in peach. Under it, the
diff itself, read-only, over the object's YAML before → after: the change stands in place
of the worktree's until the write is decided, then the worktree's is read again. No sheet
stands over it: Esc, Enter, the `a` key and the palette decide nothing, and closing,
deleting or reloading the session is refused until it is answered. Another session can be
selected meanwhile; back on this one, the change shows again. Enforced by
`the_diff_tab_holds_the_ask_then_the_change_whatever_tab_was_asked`,
`a_on_the_rail_neither_approves_nor_opens_a_sheet_over_the_ask`,
`only_the_band_s_approve_button_lets_a_cluster_write_through`,
`a_session_waiting_on_a_cluster_write_is_not_closed_before_it_is_answered` and
`a_review_takes_the_change_s_place_until_its_own_write_is_decided`.

**An object scales, restarts and goes from its row.** A list row's right-click offers
**scale** on a Deployment or a StatefulSet, **restart** on those and a DaemonSet, and
**delete** on every object, above the skills of its kind. Picking one turns the row into the
question. A scale shows the count the row wants between **−** and **+**, no **−** at none,
and sends the count set to the `scale` subresource. A restart stamps the pod template, as
`kubectl rollout restart` does. A delete takes the object's dependents after it. **keep**
sends nothing; the timeline says what landed. An object's tab offers no menu. Enforced by
`a_deployment_s_row_asks_its_restart_in_its_place_and_restart_sends_it`,
`keep_leaves_the_deployment_as_it_is`,
`a_scale_starts_from_the_count_the_row_wants_and_steps_before_it_is_sent`,
`a_scale_to_none_offers_no_step_down`,
`a_pod_s_row_offers_only_its_delete_asked_in_its_place`, `an_object_tab_opens_no_menu`,
`a_restart_stamps_the_pod_template_of_a_workload_and_refuses_a_pod`,
`a_scale_patches_the_scale_subresource_of_a_deployment_and_refuses_a_daemonset` and
`a_dry_run_delete_says_so_in_its_body_on_the_version_seen`.

**An object's YAML is edited from its bar.** A bar between the object's tabs and its YAML
starts with **EDIT**: until it is pressed the YAML is read-only and no key changes it.
**EDIT**, where the session may write and the context is not read-only, starts a draft from
the YAML as it stands; every key, a paste included, then edits it. **EDIT** gives its place
to **APPLY** and **DISCARD**, which throws the draft away; all are bordered words in capitals, **APPLY** and **EDIT** in peach as **Approve**
is. The draft stands while the object moves on, the bar saying so. Enforced by
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
