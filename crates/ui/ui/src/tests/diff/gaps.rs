//! The rows a gap hides, and the arrows that give them up.

use groove_controllers::workspace::{Command as Workspace, Way};
use groove_controllers::{AppState, Command};

use super::{on_diff, view_of, with_files};
use crate::Ui;
use crate::hit::Target;
use crate::input::{Input, handle};
use crate::tests::window;

/// A file changed at both ends, so one gap stands between the two hunks.
fn far_apart() -> AppState {
    let before: String = (0..60).map(|at| format!("line {at}\n")).collect();
    let after = before
        .replace("line 0\n", "LINE 0\n")
        .replace("line 59\n", "LINE 59\n");
    let mut app = with_files();
    crate::tests::changed_files(&mut app, &[("src/lib.rs", &before, &after)]);
    app
}

fn gap_row(app: &AppState) -> usize {
    (0..app.workspace.changes.rows())
        .find(|row| {
            matches!(
                app.workspace.changes.at(*row),
                Some(groove_controllers::workspace_service::At::Row(file, at))
                    if matches!(file.rows[at].kind, groove_types::RowKind::Gap(_))
            )
        })
        .expect("a gap stands")
}

#[test]
fn a_gap_offers_both_its_ends_and_the_whole_of_it() {
    let app = far_apart();
    let mut ui = on_diff();
    ui.session.diff = 0.0;
    let (_, hits) = view_of(&app, &ui);
    let row = gap_row(&app);

    for way in [Way::Up, Way::Down, Way::All] {
        let target = Target::Gap { row, way };
        let rect = hits
            .rect_of(&target)
            .unwrap_or_else(|| panic!("{way:?} is on screen"));
        let asked = handle(
            Input::Press {
                x: rect.x + rect.w / 2.0,
                y: rect.y + rect.h / 2.0,
                mods: Default::default(),
            },
            &mut ui,
            &app,
            &hits,
            window(),
        );
        assert_eq!(
            asked,
            [Command::Workspace(Workspace::OpenGap { row, way })],
            "{way:?}"
        );
    }
}

#[test]
fn the_arrows_stand_where_the_numbers_would() {
    let app = far_apart();
    let ui = Ui {
        session: on_diff().session,
        ..Ui::default()
    };
    let (_, hits) = view_of(&app, &ui);
    let row = gap_row(&app);
    let up = hits
        .rect_of(&Target::Gap { row, way: Way::Up })
        .expect("up");
    let down = hits
        .rect_of(&Target::Gap {
            row,
            way: Way::Down,
        })
        .expect("down");
    assert_eq!(up.y, down.y, "both stand on the row itself");
    assert!(
        down.right() <= up.x,
        "down where the old line would be, up after it"
    );
    assert!(
        up.right() <= hits.chars().left,
        "both stand where the numbers do"
    );
}
