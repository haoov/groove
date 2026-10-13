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
pub async fn new_day(dir: &Path, today: &str) -> Result<bool, Error> {
    let path = dir.join("day");
    let held = tokio::fs::read_to_string(&path).await;
    if held.is_ok_and(|day| day.trim() == today) {
        return Ok(false);
    }
    let io = |e: std::io::Error| Error::new(ErrorKind::Io, e.to_string());
    tokio::fs::create_dir_all(dir).await.map_err(io)?;
    tokio::fs::write(&path, today).await.map_err(io)?;
    Ok(true)
}
