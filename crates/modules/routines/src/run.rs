//! What a run asks its agent, and whether today's first look has come.

use std::path::Path;

use groove_types::{Error, ErrorKind, Routine, Trigger};

/// Its one skill alone, or the routine's words with what started it and its skills named.
pub fn prompt(routine: &Routine, by: Option<Trigger>, about: &str) -> String {
    if routine.words.is_empty()
        && let [skill] = routine.skills.as_slice()
    {
        return format!("/{skill} {about}").trim_end().to_string();
    }
    let by = started_by(by);
    let mut said = vec![format!("Routine `{}`, started by {by}.", routine.name)];
    if !about.is_empty() {
        said.push(format!("It is about: {about}."));
    }
    if !routine.skills.is_empty() {
        let skills: Vec<String> = routine.skills.iter().map(|one| format!("/{one}")).collect();
        said.push(format!("Use the skills {}.", skills.join(", ")));
    }
    said.push(String::new());
    said.push(routine.words.clone());
    said.join("\n")
}

/// What started a run: its trigger's name, or its button.
pub fn started_by(by: Option<Trigger>) -> &'static str {
    by.map_or("its button", |one| one.name())
}

/// Whether `today` is a new day for `<dir>/day`, which then holds it.
pub fn new_day(dir: &Path, today: &str) -> Result<bool, Error> {
    let path = dir.join("day");
    if std::fs::read_to_string(&path).is_ok_and(|day| day.trim() == today) {
        return Ok(false);
    }
    let written = std::fs::create_dir_all(dir).and_then(|()| std::fs::write(&path, today));
    written.map_err(|e| Error::new(ErrorKind::Io, e.to_string()))?;
    Ok(true)
}
