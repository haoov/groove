use groove_gfx::{Rect, Size};
use groove_ui_kit::base::tokens::Tokens;

use crate::layout::{Layout, Split};

#[test]
fn no_region_takes_a_negative_size_in_a_window_shorter_than_its_bands() {
    let tokens = Tokens::new(1.0);
    for (w, h) in [(0, 0), (120, 40), (900, 60), (1280, 130)] {
        for sidebar in [false, true] {
            let held = Layout::new(Size::new(w, h), &tokens, Split::default(), sidebar);
            let regions: [(&str, Rect); 10] = [
                ("rail", held.rail),
                ("agent", held.agent),
                ("header", held.header),
                ("workspace", held.workspace),
                ("agent_bar", held.agent_bar),
                ("sidebar", held.sidebar),
                ("commit", held.commit),
                ("feed", held.feed),
                ("manual", held.manual),
                ("board", held.board),
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
    let tokens = Tokens::new(1.0);
    let runs = 2000;
    let started = std::time::Instant::now();
    for _ in 0..runs {
        let _ = Layout::new(Size::new(1280, 800), &tokens, Split::default(), true);
    }
    println!("TIMED {:?} a cut", started.elapsed() / runs);
}
