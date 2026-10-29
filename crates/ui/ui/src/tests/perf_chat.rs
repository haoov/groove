//! What a frame of the agent's conversation costs. Run with
//! `cargo test --release -p groove-ui perf_chat -- --ignored --nocapture`.
#![allow(clippy::print_stdout)]

use std::time::{Duration, Instant};

use groove_controllers::AppState;
use groove_gfx::{Fonts, Renderer, Size};
use groove_types::chat::{Heard, Outcome, Patch};
use groove_types::{AgentStatus, Timestamp};

use super::chat::{chat_ui, talking};
use crate::tests::window;
use crate::{Ui, view};

const SAID: &str = "I add a `PriceCache` keyed by route and read its TTL from settings.\n\n\
- the cache sits in front of the client\n- a failed fetch is never cached\n\n\
| route | hits | misses |\n|---|---|---|\n| /quote | 120 | 4 |\n| /fare | 88 | 12 |\n\n\
```rust\nlet cached = self.cache.get(route);\n```\n\nTests pass.";

/// One turn: the prompt, a glance of thought, looks, commands and a call, the answer, an edit.
fn turn(at: usize) -> Vec<Heard> {
    let id = |kind: &str, n: usize| format!("{kind}-{at}-{n}");
    let mut out = vec![
        Heard::Clock(Timestamp::new(at as i64 * 60)),
        Heard::You(format!(
            "turn {at}: cache quotes per route for five minutes"
        )),
        Heard::Thought,
    ];
    for n in 0..3 {
        let at = format!("file_{n}.py");
        out.push(Heard::Look(at));
    }
    for n in 0..4 {
        let ran = Heard::Ran {
            id: id("b", n),
            command: format!("uv run pytest tests/pricing -q -k case_{n}"),
            about: Some("Run the pricing tests".into()),
        };
        let output: Vec<String> = (0..30)
            .map(|line| format!("line {line} of the output"))
            .collect();
        let printed = Outcome {
            output: Some(output.join(
                "
",
            )),
            ..Outcome::default()
        };
        out.extend([
            ran,
            Heard::Result {
                id: id("b", n),
                outcome: printed,
            },
        ]);
    }
    let call = Heard::Tool {
        id: id("g", 0),
        name: "mcp__groove__git_commit".into(),
        detail: Some("feat(pricing): cache quotes".into()),
    };
    let patch = Some(Patch {
        added: 14,
        removed: 3,
        line: 16,
    });
    let edit = Heard::Edit {
        id: id("e", 0),
        path: "/w/pricing/service.py".into(),
        created: false,
    };
    let wrote = Heard::Result {
        id: id("e", 0),
        outcome: Outcome {
            patch,
            ..Outcome::default()
        },
    };
    out.extend([call, Heard::Said(SAID.into()), edit, wrote]);
    out
}

/// How far back a long scroll goes, in pixels.
const DEEP: f32 = 200_000.0;

fn chatty(turns: usize) -> AppState {
    talking((0..turns).flat_map(turn).collect())
}

fn timed(app: &AppState, ui: &Ui, fonts: &mut Fonts, runs: u32) -> Duration {
    let _ = view(app, ui, window(), fonts);
    let started = Instant::now();
    for _ in 0..runs {
        let _ = view(app, ui, window(), fonts);
    }
    started.elapsed() / runs
}

#[test]
#[ignore]
fn time_a_frame_of_the_conversation() {
    let mut fonts = Fonts::embedded();
    for turns in [10, 100, 1000] {
        let mut app = chatty(turns);
        let mut ui = chat_ui();
        let bottom = timed(&app, &ui, &mut fonts, 20);
        ui.agent.scroll = DEEP;
        let middle = timed(&app, &ui, &mut fonts, 20);
        ui.agent.scroll = 0.0;
        app.agent.agents[0].1.activity.status = AgentStatus::Working;
        let working = timed(&app, &ui, &mut fonts, 20);
        ui.agent.terminal = true;
        let terminal = timed(&app, &ui, &mut fonts, 20);
        println!(
            "{turns:>5} turns: {bottom:>10?} at the end, {middle:>10?} scrolled far back, \
             {working:>10?} working, {terminal:>10?} the terminal instead"
        );
    }
}

#[test]
#[ignore]
fn time_the_draw_of_the_conversation() {
    let size = Size::new(crate::tests::WINDOW.0, crate::tests::WINDOW.1);
    let mut renderer = Renderer::headless(size, Fonts::embedded()).expect("a GPU adapter");
    let app = chatty(100);
    for terminal in [false, true] {
        let mut ui = chat_ui();
        ui.agent.terminal = terminal;
        for _ in 0..3 {
            let (frame, _) = view(&app, &ui, window(), renderer.fonts());
            renderer.render(&frame).expect("rendered");
        }
        let runs = 200;
        let started = Instant::now();
        for at in 0..runs {
            ui.agent.scroll = (at % 20) as f32 * 7.0;
            let (frame, _) = view(&app, &ui, window(), renderer.fonts());
            renderer.render(&frame).expect("rendered");
        }
        let face = if terminal { "terminal" } else { "conversation" };
        println!(
            "100 turns, the {face}: {:?} a frame",
            started.elapsed() / runs
        );
    }
}
