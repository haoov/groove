//! The Resources list: its colours, its order, its column widths, and what a frame of it costs.

use groove_controllers::AppState;
use groove_controllers::cluster_service::Batch;
use groove_types::{ObjectRow, SessionId, TableColumn};

use super::{key, listing, on, row};
use crate::Ui;
use crate::hit::Target;
use crate::tests::settings::drawn;
use crate::tests::{click, window};

/// Run with `cargo test --release -p groove-ui time_a_frame_of -- --ignored --nocapture`.
#[test]
#[ignore]
#[allow(clippy::print_stdout)]
fn time_a_frame_of_the_list_over_30k_rows() {
    let (mut app, mut ui) = listing();
    let open = app
        .session
        .get_mut(&SessionId::new("a"))
        .expect("the session");
    open.clusters = ["a", "b", "c"]
        .iter()
        .map(|one| on("staging", Some(one)))
        .collect();
    let names = ["Name", "Ready", "Status", "Restarts", "Age"];
    for namespace in ["a", "b", "c"] {
        let key = key("staging", Some(namespace));
        app.cluster.store.lease(&key, "resources");
        let rows = (0..10_000)
            .map(|at| ObjectRow {
                cells: vec![
                    format!("api-{at:05}-7d9f8c6b5-x2k4p"),
                    "1/1".into(),
                    "Running".into(),
                    "0".into(),
                    format!("{}m", at % 600),
                ],
                ..row(&format!("api-{at:05}-7d9f8c6b5-x2k4p"), namespace, "")
            })
            .collect();
        let columns = names
            .iter()
            .map(|name| TableColumn {
                name: (*name).into(),
                priority: 0,
                date: false,
            })
            .collect();
        app.cluster
            .store
            .apply(&key, Batch::Reset { columns, rows });
    }
    let mut fonts = groove_gfx::Fonts::embedded();
    let sorts = [None, Some(("NAME", false)), Some(("AGE", true))];
    for (search, sort) in [
        ("", sorts[0]),
        ("api-01", sorts[0]),
        ("", sorts[1]),
        ("", sorts[2]),
    ] {
        ui.session.resources.search.set(search);
        ui.session.resources.sort = sort.map(|(label, down)| (label.to_string(), down));
        let _ = crate::view(&app, &ui, window(), &mut fonts);
        let started = std::time::Instant::now();
        for _ in 0..10 {
            let _ = crate::view(&app, &ui, window(), &mut fonts);
        }
        let kept = started.elapsed() / 10;
        let started = std::time::Instant::now();
        for _ in 0..10 {
            ui.session.resources.kept = Default::default();
            let _ = crate::view(&app, &ui, window(), &mut fonts);
        }
        println!(
            "30k rows, search {search:?}, sort {sort:?}: {kept:?} a frame, {:?} rebuilt",
            started.elapsed() / 10
        );
    }
}

#[test]
fn a_status_and_a_ready_count_wear_the_colour_of_how_the_object_stands() {
    use groove_ui_kit::base::style::{Role, Styles};
    let (app, ui) = listing();
    let (frame, _) = crate::view(&app, &ui, window(), &mut groove_gfx::Fonts::embedded());
    let styles = Styles::new(app.config.theme(), window().tokens());
    let colour = |text: &str| {
        let runs = frame.layers().iter().flat_map(|layer| layer.texts.iter());
        runs.filter(|one| one.text == text)
            .map(|one| one.style.color)
            .next()
    };
    assert_eq!(colour("CrashLoopBackOff"), Some(styles.color(Role::Bad)));
    assert_eq!(colour("Running"), Some(styles.color(Role::Ok)));
    assert_eq!(colour("1/1"), Some(styles.color(Role::Ok)));
}

#[test]
fn a_header_orders_the_rows_by_its_column_and_again_the_other_way() {
    let (app, mut ui) = listing();
    let (_, hits) = drawn(&app, &ui);
    let status = hits
        .rect_of(&Target::ResourceSort("STATUS".into()))
        .expect("the status header");
    click(status, &mut ui, &app, &hits);
    assert_eq!(ui.session.resources.sort, Some(("STATUS".into(), false)));
    let order = |app: &AppState, ui: &Ui| {
        let (texts, _) = drawn(app, ui);
        let names = ["api-0", "cnpg-1", "worker-0"];
        texts
            .into_iter()
            .filter(|one| names.contains(&one.as_str()))
            .collect::<Vec<_>>()
    };
    assert_eq!(order(&app, &ui), ["worker-0", "api-0", "cnpg-1"]);
    let (_, hits) = drawn(&app, &ui);
    let status = hits
        .rect_of(&Target::ResourceSort("STATUS".into()))
        .expect("the status header");
    click(status, &mut ui, &app, &hits);
    assert_eq!(order(&app, &ui), ["api-0", "cnpg-1", "worker-0"]);
}

#[test]
fn a_column_s_edge_drags_its_width_kept_for_the_kind_and_drawn_wider() {
    let (app, mut ui) = listing();
    let (_, hits) = drawn(&app, &ui);
    let edge = hits
        .rect_of(&Target::ResourceEdge("READY".into()))
        .expect("the ready edge");
    let header = hits
        .rect_of(&Target::ResourceSort("READY".into()))
        .expect("the ready header");
    let (x, y) = (edge.x + 1.0, edge.y + 1.0);
    crate::input::handle(
        crate::input::Input::Press {
            x,
            y,
            mods: Default::default(),
        },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    crate::input::handle(
        crate::input::Input::Move { x: x + 40.0, y },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    crate::input::handle(crate::input::Input::Release, &mut ui, &app, &hits, window());
    let wide = ui
        .session
        .resources
        .widths
        .get("pods")
        .and_then(|one| one.get("READY"))
        .copied();
    let moved = header.w + 40.0;
    assert!(
        wide.is_some_and(|wide| (wide - moved).abs() < 0.01),
        "{wide:?} {moved}"
    );
    let (_, hits) = drawn(&app, &ui);
    let after = hits
        .rect_of(&Target::ResourceSort("READY".into()))
        .expect("the ready header");
    assert!(
        after.w > header.w,
        "{} drawn wider than {}",
        after.w,
        header.w
    );
}

#[test]
fn a_ready_count_of_a_completed_row_stays_faint() {
    use groove_ui_kit::base::style::{Role, Styles};
    let (mut app, ui) = listing();
    let key = key("staging", Some("cnpg"));
    let done = ObjectRow {
        cells: vec!["migrate".into(), "0/1".into(), "Completed".into()],
        ..row("migrate", "cnpg", "")
    };
    app.cluster.store.apply(
        &key,
        Batch::Changes(vec![groove_controllers::cluster_service::Delta::Put(done)]),
    );
    let (frame, _) = crate::view(&app, &ui, window(), &mut groove_gfx::Fonts::embedded());
    let styles = Styles::new(app.config.theme(), window().tokens());
    let runs = frame.layers().iter().flat_map(|layer| layer.texts.iter());
    let ready = runs
        .filter(|one| one.text == "0/1")
        .map(|one| one.style.color)
        .next();
    assert_eq!(ready, Some(styles.color(Role::Ghost)));
}

#[test]
fn a_table_narrower_than_its_columns_shrinks_them_all_and_overflows_nothing() {
    let (app, mut ui) = listing();
    let wide: std::collections::BTreeMap<String, f32> = ["NAME", "READY", "STATUS"]
        .iter()
        .map(|one| (one.to_string(), 2000.0))
        .collect();
    ui.session.resources.widths.insert("pods".into(), wide);
    let (_, hits) = drawn(&app, &ui);
    let list = hits
        .rect_of(&Target::ResourceRow("paxone/api-0".into()))
        .expect("a row");
    let status = hits
        .rect_of(&Target::ResourceSort("STATUS".into()))
        .expect("the status header");
    assert!(
        status.right() <= list.right() + 0.5,
        "{} past {}",
        status.right(),
        list.right()
    );
}

#[test]
fn an_age_due_asks_a_tick_while_the_list_shows_and_the_list_reads_it_after() {
    use groove_controllers::Command;
    use groove_controllers::cluster::Command as Cluster;
    use groove_types::{Aging, Timestamp};
    let (mut app, mut ui) = listing();
    let key = key("staging", Some("paxone"));
    let young = ObjectRow {
        aging: vec![Aging {
            cell: 1,
            since: Timestamp::new(1_000),
            said: 0,
            lead: None,
            turn: Timestamp::default(),
        }],
        ..row("young", "paxone", "Running")
    };
    app.cluster.store.apply(
        &key,
        Batch::Changes(vec![groove_controllers::cluster_service::Delta::Put(young)]),
    );
    let now = Timestamp::new(1_179);
    let at = crate::Metrics { now, ..window() };
    let tick = Command::Cluster(Cluster::Age { now });
    assert!(crate::frame_commands(&app, &ui, at).contains(&tick));
    app.cluster.store.age(now);
    assert!(
        !crate::frame_commands(&app, &ui, at).contains(&tick),
        "not before its next turn"
    );
    assert_eq!(crate::ages_due(&app, &ui), Some(Timestamp::new(1_180)));
    assert!(drawn(&app, &ui).0.iter().any(|one| one == "2m59s"));
    ui.session.tab = crate::Tab::Diff;
    assert_eq!(
        crate::ages_due(&app, &ui),
        None,
        "no wake while the list is hidden"
    );
}

#[test]
fn the_kept_order_gives_way_to_a_row_that_lands_and_to_a_sort_picked() {
    let (mut app, mut ui) = listing();
    ui.session.resources.sort = Some(("NAME".into(), false));
    let first = |app: &AppState, ui: &Ui| {
        let (texts, _) = drawn(app, ui);
        let names = ["aaa-new", "api-0", "worker-0", "cnpg-1"];
        texts.into_iter().find(|one| names.contains(&one.as_str()))
    };
    assert_ne!(first(&app, &ui).as_deref(), Some("aaa-new"));
    let put = groove_controllers::cluster_service::Delta::Put(row("aaa-new", "paxone", "Running"));
    app.cluster
        .store
        .apply(&key("staging", Some("paxone")), Batch::Changes(vec![put]));
    assert_eq!(
        first(&app, &ui).as_deref(),
        Some("aaa-new"),
        "the new row, in its place"
    );
    ui.session.resources.sort = Some(("NAME".into(), true));
    assert_ne!(
        first(&app, &ui).as_deref(),
        Some("aaa-new"),
        "the order turned"
    );
}
