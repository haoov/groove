//! Settings' Alerts section: the `inc` login as the last read found it, and the poll interval.

use groove_controllers::config_service::SignIn;
use groove_controllers::{Command, config};
use groove_types::Timestamp;

use super::opened;
use crate::hit::Target;
use crate::tests::{click, drawn};
use crate::views::settings::Section;

#[test]
fn alerts_show_the_inc_login_and_its_poll_and_log_in_runs_inc_auth_login() {
    let (mut app, mut ui) = opened();
    ui.settings.section = Section::Alerts;
    app.alert.failed = Some("no token for api.incident.io: run `inc auth login`".into());
    let (texts, hits) = drawn(&app, &ui);
    for shown in [
        "no token for api.incident.io: run `inc auth login`",
        "60 seconds",
    ] {
        assert!(texts.iter().any(|t| t == shown), "{shown}: {texts:?}");
    }
    let log_in = hits
        .rect_of(&Target::SettingsIncLogin)
        .expect("the inc login");
    let login = config::Command::Login {
        to: SignIn::Inc,
        cols: 80,
        rows: 24,
    };
    assert_eq!(
        click(log_in, &mut ui, &app, &hits),
        [Command::Config(login)]
    );

    app.alert.failed = None;
    app.alert.read_at = Some(Timestamp::now());
    let (texts, _) = drawn(&app, &ui);
    assert!(texts.iter().any(|t| t == "logged in"), "{texts:?}");
}
