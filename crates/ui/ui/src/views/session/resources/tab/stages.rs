//! A sync's stages: the hooks before the waves, each wave in order, the hooks after, and how far each step got.

use std::collections::BTreeMap;

use groove_types::{AppPart, AppResource, Operation, SyncedResource};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum State {
    Failed,
    Applying,
    Waiting,
    Done,
    /// The sync ended before it.
    Skipped,
}

#[derive(Debug)]
pub(crate) struct Step<'a> {
    pub kind: &'a str,
    pub name: &'a str,
    pub state: State,
    pub message: &'a str,
    /// A hook run inside a wave, which its row says.
    pub in_wave: bool,
}

#[derive(Debug)]
pub(crate) struct Stage<'a> {
    pub title: String,
    pub steps: Vec<Step<'a>>,
}

impl Stage<'_> {
    pub(super) fn count(&self, state: State) -> usize {
        self.steps.iter().filter(|one| one.state == state).count()
    }

    pub(super) fn settled(&self) -> bool {
        let busy = [State::Failed, State::Applying, State::Waiting];
        busy.into_iter().all(|state| self.count(state) == 0)
    }
}

/// The hooks before the waves, each wave in order with its Sync hooks, then the hooks after.
pub(crate) fn stages(part: &AppPart) -> Vec<Stage<'_>> {
    let Some(operation) = &part.operation else {
        return Vec::new();
    };
    let hooks = |phase: &str| Stage {
        title: format!("{phase} hooks"),
        steps: operation
            .applied
            .iter()
            .filter(|one| one.hook && one.phase == phase)
            .map(|one| hooked(operation, one))
            .collect(),
    };
    let waves = waves(part, operation);
    let waved = waves.into_iter().map(|(wave, steps)| Stage {
        title: format!("wave {wave}"),
        steps,
    });
    let mut out = vec![hooks("PreSync")];
    out.extend(waved);
    out.extend(["PostSync", "SyncFail"].map(hooks));
    out.retain(|one| !one.steps.is_empty());
    for stage in &mut out {
        stage.steps.sort_by_key(|one| one.state);
    }
    out
}

/// Each wave's resources, and the Sync hooks at the wave the Application lists them, else 0.
fn waves<'a>(part: &'a AppPart, operation: &'a Operation) -> BTreeMap<i64, Vec<Step<'a>>> {
    let mut waves: BTreeMap<i64, Vec<Step>> = BTreeMap::new();
    for one in part.resources.iter().filter(|one| !one.hook) {
        let result = operation
            .applied
            .iter()
            .find(|done| !done.hook && one.is(done));
        let state = match result {
            Some(done) if done.broke() => State::Failed,
            Some(done) if operation.running() && waits(one, done) => State::Applying,
            Some(_) => State::Done,
            None if operation.running() => State::Waiting,
            None => State::Skipped,
        };
        let message = result.map_or(one.message.as_str(), |done| done.message.as_str());
        let step = Step {
            kind: &one.kind,
            name: &one.name,
            state,
            message,
            in_wave: false,
        };
        waves.entry(one.wave).or_default().push(step);
    }
    let hooks = operation
        .applied
        .iter()
        .filter(|one| one.hook && one.phase == "Sync");
    for one in hooks {
        let listed = part.resources.iter().find(|held| held.hook && held.is(one));
        let wave = listed.map_or(0, |held| held.wave);
        let step = Step {
            in_wave: true,
            ..hooked(operation, one)
        };
        waves.entry(wave).or_default().push(step);
    }
    waves
}

fn hooked<'a>(operation: &Operation, one: &'a SyncedResource) -> Step<'a> {
    let state = match one.hook_phase.as_str() {
        _ if one.broke() => State::Failed,
        "Running" | "Pending" if operation.running() => State::Applying,
        "Running" | "Pending" => State::Skipped,
        _ => State::Done,
    };
    Step {
        kind: &one.kind,
        name: &one.name,
        state,
        message: &one.message,
        in_wave: false,
    }
}

/// Applied, and not done yet: its own phase runs, or its health still progresses.
fn waits(one: &AppResource, done: &SyncedResource) -> bool {
    done.hook_phase == "Running" || one.health == "Progressing"
}

/// How many steps the running sync has still to apply; none once it ends.
pub(super) fn remaining(part: &AppPart) -> usize {
    match &part.operation {
        Some(one) if one.running() => stages(part)
            .iter()
            .flat_map(|stage| &stage.steps)
            .filter(|one| one.state != State::Done)
            .count(),
        _ => 0,
    }
}
