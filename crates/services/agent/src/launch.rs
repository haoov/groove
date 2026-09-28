//! Starting an agent for a session, and what its terminal reports.

use std::path::{Path, PathBuf};

use groove_agent_launch::{Launch, Loopback, Paths, Tools};
use groove_hooks::Receiver;
use groove_mcp::Server;
use groove_terminal::{Hooks, Terminal};
use groove_types::{AnsiPalette, Error, Session, ThemeName};

/// Where a launch reads and writes, from the app's environment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaunchPaths {
    pub home: PathBuf,
    pub launch_dir: PathBuf,
    pub plugin_dirs: Vec<PathBuf>,
    /// The loopback the agent's hooks post to, when one is listening.
    pub hooks: Option<Receiver>,
    /// The loopback the agent asks its tools of, when one is listening.
    pub tools: Option<Server>,
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
        loopback(paths, session).as_ref(),
    )?;
    let hooks = Hooks { on_damage, on_exit };
    Terminal::spawn(launch.spec(cols, rows), palette, hooks)
        .map_err(|e| Error::new(groove_types::ErrorKind::Agent, e.to_string()))
}

/// `claude auth login` on a terminal of `cols` by `rows`.
pub fn login(
    home: &Path,
    (cols, rows): (u16, u16),
    palette: AnsiPalette,
    hooks: Hooks,
) -> Result<Terminal, Error> {
    let spec = groove_agent_launch::login(home, cols, rows);
    Terminal::spawn(spec, palette, hooks)
        .map_err(|e| Error::new(groove_types::ErrorKind::Agent, e.to_string()))
}

/// The `claude` a launch runs.
pub fn claude_bin(home: &Path) -> String {
    groove_agent_launch::claude_bin(home)
}

/// Every session posts its hooks to its own url, and asks its tools on its own stream.
pub(crate) fn loopback(paths: &LaunchPaths, session: &Session) -> Option<Loopback> {
    let hooks = paths.hooks.as_ref()?;
    Some(Loopback {
        tools: paths.tools.as_ref().map(|tools| Tools {
            sse_url: tools.sse_url(session.id.as_str()),
            token: tools.token.clone(),
        }),
        hook_url: hooks.hook_url(session.id.as_str()),
        token: hooks.token.clone(),
    })
}

/// The terminal colours that go with the theme.
pub fn palette(theme: ThemeName) -> AnsiPalette {
    match theme {
        ThemeName::Latte => AnsiPalette::LATTE,
        ThemeName::Frappe => AnsiPalette::FRAPPE,
        ThemeName::Macchiato => AnsiPalette::MACCHIATO,
        ThemeName::Mocha => AnsiPalette::MOCHA,
    }
}
