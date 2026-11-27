use groove_gfx::Rect;

use crate::Ui;
use crate::layout::Layout;

#[test]
fn no_region_takes_a_negative_size_in_a_window_shorter_than_its_bands() {
    for (w, h) in [(0, 0), (120, 40), (900, 60), (1280, 130)] {
        for folded in [false, true] {
            let mut ui = Ui::default();
            (ui.session.folded, ui.session.manual) = (folded, !folded);
            let held = Layout::of(crate::tests::metrics(w, h, 1.0), &ui);
            let regions: [(&str, Rect); 9] = [
                ("rail", held.rail),
                ("sessions", held.sessions),
                ("agent", held.agent),
                ("top", held.top),
                ("sidebar", held.sidebar),
                ("feed", held.feed),
                ("manual", held.manual),
                ("board", held.board),
                ("lists", held.lists),
            ];
            for (name, rect) in regions {
                assert!(
                    rect.w >= 0.0 && rect.h >= 0.0 && rect.y >= 0.0,
                    "{name} at {w}x{h}: {rect:?}"
                );
            }
        }
    }
}

#[test]
#[ignore]
#[allow(clippy::print_stdout)]
fn time_cutting_the_window() {
    let runs = 2000;
    let ui = Ui::default();
    let started = std::time::Instant::now();
    for _ in 0..runs {
        let _ = Layout::of(crate::tests::window(), &ui);
    }
    println!("TIMED {:?} a cut", started.elapsed() / runs);
}
