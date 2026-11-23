use groove_gfx::Rect;

use super::{Dice, LEVELS, Lattice, PERIOD, WIDEST, grown};

fn lattice() -> Lattice {
    Lattice::of(Rect::new(0.0, 40.0, 900.0, 600.0), 18.0)
}

#[test]
fn every_hop_runs_the_lattice_s_edges_and_never_climbs() {
    let lattice = lattice();
    for seed in 0..200 {
        let (hops, _) = grown(&lattice, &mut Dice(seed));
        for hop in &hops {
            assert!(hop.path.len() > 1, "a hop goes somewhere");
            for w in hop.path.windows(2) {
                let ((x0, y0), (x1, y1)) = (w[0], w[1]);
                let edge = (x1 - x0).hypot(y1 - y0);
                assert!((edge - 18.0).abs() < 0.01, "one hex side at a time: {edge}");
                assert!(y1 >= y0 - 9.01, "down or along a row: {y0} → {y1}");
            }
        }
    }
}

#[test]
fn a_cascade_starts_anywhere_branches_and_stays_inside() {
    let lattice = lattice();
    let rect = Rect::new(0.0, 40.0, 900.0, 600.0);
    for seed in 0..200 {
        let (hops, fired) = grown(&lattice, &mut Dice(seed));
        assert_eq!(fired[0].1, 0, "the root fires first");
        assert!(hops.len() >= 2, "the root splits in two");
        assert!(hops.len() <= 2 + WIDEST * (LEVELS as usize - 1));
        assert!(
            fired.iter().all(|(_, at)| *at < PERIOD),
            "all fire within its life"
        );
        for (corner, _) in &fired {
            let (x, y) = lattice.at(*corner);
            assert!(x >= rect.x && x <= rect.right(), "across: {x}");
            assert!(y >= rect.y && y <= rect.bottom(), "down: {y}");
        }
    }
}

#[test]
fn the_same_seed_grows_the_same_cascade() {
    let lattice = lattice();
    let paths = |seed| {
        let (hops, _) = grown(&lattice, &mut Dice(seed));
        hops.into_iter().map(|hop| hop.path).collect::<Vec<_>>()
    };
    assert_eq!(paths(7), paths(7));
    assert_ne!(paths(7), paths(8));
}

fn unders_at(tick: u64) -> Vec<(Rect, groove_gfx::Color, groove_gfx::Shape)> {
    let (_, frame) = crate::tests::drawn::drawn(|ctx| {
        ctx.tick = tick;
        super::neurons(ctx, Rect::new(0.0, 40.0, 900.0, 600.0));
    });
    frame.layers()[0].under.clone()
}

#[test]
fn signals_light_the_lattice_on_the_ground_and_fire_its_corners() {
    let drawn = unders_at(1900);
    assert!(
        drawn
            .iter()
            .any(|(_, _, shape)| shape.lattice > 0.0 && shape.blur > 0.0),
        "the lattice lit, fading out from each signal"
    );
    assert!(
        drawn
            .iter()
            .any(|(_, _, shape)| shape.lattice == 0.0 && shape.radius > 0.0),
        "a corner fired"
    );
}

#[test]
fn a_tick_draws_the_same_signals_each_time() {
    assert_eq!(unders_at(5321), unders_at(5321));
    assert_ne!(unders_at(5321), unders_at(5361), "and they move");
}

#[test]
fn cascades_start_on_many_rows_not_only_the_top() {
    let lattice = lattice();
    let rows: std::collections::HashSet<i32> = (0..50)
        .map(|seed| grown(&lattice, &mut Dice(seed)).1[0].0.l)
        .collect();
    assert!(rows.len() > 5, "roots on {rows:?}");
}
