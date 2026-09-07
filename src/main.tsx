import ReactDOM from 'react-dom/client';
// Bundled fonts: IBM Plex Sans for the UI chrome, Lilex for the editor and code.
import '@fontsource/lilex/latin-300.css';
import '@fontsource/lilex/latin-ext-300.css';
import '@fontsource/lilex/latin-400.css';
import '@fontsource/lilex/latin-ext-400.css';
import '@fontsource/lilex/latin-500.css';
import '@fontsource/lilex/latin-ext-500.css';
import '@fontsource/lilex/latin-600.css';
import '@fontsource/lilex/latin-ext-600.css';
import '@fontsource/lilex/latin-700.css';
import '@fontsource/lilex/latin-ext-700.css';
import '@fontsource/ibm-plex-sans/latin-400.css';
import '@fontsource/ibm-plex-sans/latin-ext-400.css';
import '@fontsource/ibm-plex-sans/latin-500.css';
import '@fontsource/ibm-plex-sans/latin-ext-500.css';
import '@fontsource/ibm-plex-sans/latin-600.css';
import '@fontsource/ibm-plex-sans/latin-ext-600.css';
import '@fontsource/ibm-plex-sans/latin-700.css';
import '@fontsource/ibm-plex-sans/latin-ext-700.css';
import '@fontsource/ibm-plex-mono/latin-400.css';
import '@fontsource/ibm-plex-mono/latin-ext-400.css';
import '@fontsource/ibm-plex-mono/latin-500.css';
import '@fontsource/ibm-plex-mono/latin-ext-500.css';
import '@fontsource/ibm-plex-mono/latin-600.css';
import '@fontsource/ibm-plex-mono/latin-ext-600.css';
import '@fontsource/ibm-plex-mono/latin-700.css';
import '@fontsource/ibm-plex-mono/latin-ext-700.css';
import './shared/styles/global.css';
import './app/layout.css';
import './home/home.css';
import './agent/console.css';
import './workspace/sidebar.css';
import './files/files.css';
import './git/git.css';
import './notes/notes.css';
import './notifications/notifications.css';
import './setup/setup.css';
import './settings/settings.css';
import './command/command.css';
import './approvals/approvals.css';
import './overview/explorer.css';
import './git/diff.css';
import './editor/editor.css';
import './workspace/workspace.css';
import './agent/agent.css';
import './agent/agents.css';
import './notifications/feed.css';
import './overview/overview.css';
import './setup/firstrun.css';
import { initPlatform } from './shared/lib/actions/platform';
import { AGENT_WINDOW_LABEL } from './shared/lib/pure/agentWindow';

// initPlatform runs before the store module builds the default keymap. The window label picks the root.
initPlatform().then(async () => {
  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  const { default: Root } = getCurrentWindow().label === AGENT_WINDOW_LABEL
    ? await import('./app/AgentWindow')
    : await import('./app/App');
  ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
    <Root />
  );
});
