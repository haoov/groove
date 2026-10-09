//! A value of an object's tab, copied by a click.

use groove_controllers::cluster::Command as Cluster;
use groove_controllers::{AppState, Command};
use groove_gfx::Icon;
use groove_ui_kit::base::style::{Role, Styles};

use super::tab::{crashing, lands, link, opened};
use super::{kind, pods};
use crate::Ui;
use crate::hit::Target;
use crate::tests::settings::drawn;
use crate::tests::{click, window};
use crate::views::session::resources::Link;

/// The icons drawn, each with its colour.
fn icons(app: &AppState, ui: &Ui) -> Vec<(Icon, groove_gfx::Color)> {
    let (frame, _) = crate::view(app, ui, window(), &mut groove_gfx::Fonts::embedded());
    let layers = frame.layers();
    let all = layers.iter().flat_map(|one| one.icons.iter());
    all.map(|one| (one.icon, one.color)).collect()
}

#[test]
fn a_hovered_value_shows_the_copy_mark_and_a_click_copies_it_and_ticks() {
    let (mut app, mut ui) = opened();
    lands(&mut app, &link("api-0", pods()).key(), vec![crashing()]);
    let styles = Styles::new(app.config.theme(), window().tokens());
    let copies = |icon, role| {
        move |all: &[(Icon, groove_gfx::Color)]| all.contains(&(icon, styles.color(role)))
    };
    let (faint, ok) = (
        copies(Icon::Copy, Role::Faint),
        copies(Icon::Check, Role::Ok),
    );
    assert!(
        !faint(&icons(&app, &ui)),
        "no mark before the pointer comes"
    );
    let (_, hits) = drawn(&app, &ui);
    let ip = Target::ResourceCopy("10.2.4.118".into(), None);
    let at = hits.rect_of(&ip).expect("the pod ip copies");
    crate::input::hover(&mut ui, &hits, at.x + 1.0, at.y + at.h / 2.0);
    assert!(faint(&icons(&app, &ui)), "the mark shows on hover");
    let (_, hits) = drawn(&app, &ui);
    let asked = click(hits.rect_of(&ip).expect("the pod ip"), &mut ui, &app, &hits);
    let copy = Cluster::CopyValue {
        text: "10.2.4.118".into(),
    };
    assert_eq!(asked, [Command::Cluster(copy)]);
    assert!(ok(&icons(&app, &ui)), "the mark ticks green once copied");
}

#[test]
fn a_click_on_a_link_copies_it_and_a_ctrl_click_opens_its_object() {
    let (mut app, mut ui) = opened();
    let nodes = kind("", "Node", "nodes", false);
    let mut kinds = app.cluster.store.kinds("staging").expect("kinds").to_vec();
    kinds.push(nodes.clone());
    app.cluster.store.set_kinds("staging", kinds);
    lands(&mut app, &link("api-0", pods()).key(), vec![crashing()]);
    let node = Link {
        namespace: None,
        ..link("gra9-node-2", nodes)
    };
    let target = Target::ResourceCopy("gra9-node-2".into(), Some(Box::new(node.clone())));
    let (_, hits) = drawn(&app, &ui);
    let at = hits.rect_of(&target).expect("the node copies");
    let asked = click(at, &mut ui, &app, &hits);
    let copy = Cluster::CopyValue {
        text: "gra9-node-2".into(),
    };
    assert_eq!(asked, [Command::Cluster(copy)]);
    assert_eq!(
        ui.session.resources.opened.len(),
        1,
        "a click opens nothing"
    );
    let ctrl = crate::input::Modifiers {
        ctrl: true,
        ..Default::default()
    };
    let press = crate::input::Input::Press {
        x: at.x + 1.0,
        y: at.y + at.h / 2.0,
        mods: ctrl,
    };
    crate::input::handle(press, &mut ui, &app, &hits, window());
    let shown = ui.session.resources.tab().map(|one| &one.link);
    assert_eq!(shown, Some(&node), "a ctrl+click opens the node");
}
