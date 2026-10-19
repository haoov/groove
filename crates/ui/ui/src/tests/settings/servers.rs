use groove_controllers::config_service::Preference;
use groove_controllers::{Command, config};

use super::{click, drawn, opened};
use crate::hit::Target;
use crate::views::settings::Section;

#[test]
fn each_server_says_what_it_serves_whether_it_runs_and_switches_off() {
    let (mut app, mut ui) = opened();
    ui.settings.section = Section::Servers;
    app.config.servers = vec![
        ("ty".into(), Some("0.0.89".into())),
        ("yaml-language-server".into(), None),
    ];
    let (texts, hits) = drawn(&app, &ui);
    for shown in [
        "ty",
        "python · ty server · 0.0.89",
        "yaml · yaml-language-server --stdio · not found on the login PATH",
    ] {
        assert!(texts.iter().any(|t| t == shown), "{shown}: {texts:?}");
    }
    let off = Target::SetPreference(Preference::Server("ty".into(), false));
    let switch = hits.rect_of(&off).expect("ty's switch");
    let asked = click(switch, &mut ui, &app, &hits);
    let set = config::Command::SetPreference(Preference::Server("ty".into(), false));
    assert_eq!(asked, [Command::Config(set)]);
}
