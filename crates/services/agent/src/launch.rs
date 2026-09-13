use std::path::{Path, PathBuf};

use groove_agent_launch::{Launch, Paths};
use groove_terminal::{Hooks, Terminal};
use groove_types::{AnsiPalette, Error, Session, ThemeName};

/// Where a launch reads and writes, from the app's environment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaunchPaths {
    pub home: PathBuf,
    pub launch_dir: PathBuf,
    pub plugin_dirs: Vec<PathBuf>,
}

/// Plans the command line and spawns it on a terminal of `cols` by `rows`.
pub fn launch(
    session: &Session,
    paths: &LaunchPaths,
    cwd: &Path,
    (cols, rows): (u16, u16),
    palette: AnsiPalette,
    on_damage: Box<dyn Fn() + Send + Sync>,
    on_exit: Box<dyn FnOnce(u32) + Send>,
) -> Result<Terminal, Error> {
    let launch = Launch::plan(
        session,
        &Paths {
            home: &paths.home,
            launch_dir: &paths.launch_dir,
            cwd,
        },
        &paths.plugin_dirs,
        None,
    )?;
    let hooks = Hooks { on_damage, on_exit };
    Terminal::spawn(launch.spec(cols, rows), palette, hooks)
        .map_err(|e| Error::new(groove_types::ErrorKind::Agent, e.to_string()))
}

/// The terminal colours that go with the theme.
pub fn palette(theme: ThemeName) -> AnsiPalette {
    match theme {
        ThemeName::Latte => AnsiPalette::LATTE,
        ThemeName::Mocha => AnsiPalette::MOCHA,
    }
}
