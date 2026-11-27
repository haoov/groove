//! What the composer of a reply does: sent, or dropped.

use groove_controllers::Command;

use crate::Ui;
use crate::hit::Composing;

/// One of the composer's buttons, clicked.
pub(crate) fn composing(ui: &mut Ui, asked: Composing) -> Vec<Command> {
    match asked {
        Composing::Save => {
            let act = ui.session.noting.take().and_then(|noting| noting.sent());
            act.map(Command::Delivery).into_iter().collect()
        }
        Composing::Cancel => {
            ui.session.noting = None;
            Vec::new()
        }
    }
}
