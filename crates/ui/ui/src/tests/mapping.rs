//! A source's mapping in Settings: its schema read once, each name and value picked from it.

use groove_controllers::{AppState, Command, config};
use groove_gfx::Fonts;
use groove_types::{Kind, Mapped, Mapping, Property, ProviderId, StatusIntent};

use super::*;
use crate::hit::Target;
use crate::views::settings::Section;
use crate::views::settings::rows::Slot;
use crate::{layout_commands, view};

fn property(name: &str, kind: Kind, options: &[&str]) -> Property {
    Property {
        name: name.into(),
        kind,
        options: options.iter().map(|one| one.to_string()).collect(),
    }
}

/// Notion on and read, its status mapped; GitHub off.
fn mapped() -> (AppState, Ui) {
    let mut app = crate::tests::bar::sourced(full_app(), false, true);
    if let Some(notion) = app.config.config.as_mut().and_then(|c| c.notion.as_mut()) {
        notion.properties.status = "Status".into();
    }
    app.config.schemas = vec![(
        ProviderId::Notion,
        vec![
            property("Due date", Kind::Date, &[]),
            property("Hours spent", Kind::Number, &[]),
            property("Status", Kind::Status, &["Ready", "Done"]),
            property("Created", Kind::Date, &[]),
        ],
    )];
    let mut ui = Ui::default();
    ui.settings.open = true;
    ui.settings.section = Section::Providers;
    (app, ui)
}

fn drawn(app: &AppState, ui: &Ui) -> (Vec<String>, Hits) {
    let tall = metrics(1280, 1600, 1.0);
    let (frame, hits) = view(app, ui, tall, &mut Fonts::embedded());
    let texts = frame.layers()[0]
        .texts
        .iter()
        .map(|one| one.text.clone())
        .collect();
    (texts, hits)
}

#[test]
fn a_source_shown_unread_is_read_once() {
    let (mut app, ui) = mapped();
    app.config.schemas.clear();
    let asked = layout_commands(&app, &ui, window());
    let read = Command::Config(config::Command::ReadSchema(ProviderId::Notion));
    assert!(asked.contains(&read), "{asked:?}");
    app.config.reading.push(ProviderId::Notion);
    assert!(
        !layout_commands(&app, &ui, window()).contains(&read),
        "not while it is read"
    );
}

/// The picker of `slot` clicked: the rows its menu offers.
fn opened(app: &AppState, ui: &mut Ui, slot: Slot) -> Vec<String> {
    let (_, hits) = drawn(app, ui);
    let open = Target::SettingsPick(ProviderId::Notion, slot);
    click(hits.rect_of(&open).expect("the picker"), ui, app, &hits);
    let menu = ui.menu().expect("a menu under the picker");
    let rows = crate::views::overlays::actions::rows(&menu.of);
    rows.into_iter().map(str::to_string).collect()
}

/// Row `at` of the open menu clicked: what it asks.
fn picked(app: &AppState, ui: &mut Ui, at: usize) -> Vec<Command> {
    let (_, hits) = drawn(app, ui);
    let row = hits.rect_of(&Target::MenuRow(at)).expect("the row");
    click(row, ui, app, &hits)
}

fn map(change: Mapping) -> [Command; 1] {
    let source = ProviderId::Notion;
    [Command::Config(config::Command::Map { source, change })]
}

#[test]
fn a_name_is_picked_from_a_menu_of_the_properties_of_its_type() {
    let (app, mut ui) = mapped();
    let offered = opened(&app, &mut ui, Slot::Name(Mapped::Due));
    assert_eq!(offered, ["Due date", "Created"], "a number is no date");
    let asked = picked(&app, &mut ui, 0);
    assert_eq!(asked, map(Mapping::Name(Mapped::Due, "Due date".into())));
    assert!(ui.menu().is_none(), "the menu closes");
}

#[test]
fn each_of_groove_s_statuses_is_given_one_value_of_the_status() {
    let (app, mut ui) = mapped();
    let (texts, _) = drawn(&app, &ui);
    for status in ["ready", "in progress", "done"] {
        assert!(texts.iter().any(|one| one == status), "{status}: {texts:?}");
    }
    let offered = opened(&app, &mut ui, Slot::Status(StatusIntent::Done));
    assert_eq!(offered, ["Ready", "Done"]);
    let asked = picked(&app, &mut ui, 1);
    assert_eq!(
        asked,
        map(Mapping::Status(StatusIntent::Done, "Done".into()))
    );
}

#[test]
fn a_source_not_read_yet_says_so_and_offers_nothing() {
    let (mut app, mut ui) = mapped();
    app.config.schemas.clear();
    let offered = opened(&app, &mut ui, Slot::Name(Mapped::Due));
    assert_eq!(offered, ["the source is not read yet"]);
    assert!(picked(&app, &mut ui, 0).is_empty());
}

#[test]
fn esc_closes_the_menu_and_leaves_settings_open() {
    let (app, mut ui) = mapped();
    opened(&app, &mut ui, Slot::Name(Mapped::Due));
    press(Key::Escape, Modifiers::default(), &mut ui, &app);
    assert!(ui.menu().is_none() && ui.settings.open);
}
