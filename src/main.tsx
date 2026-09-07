import ReactDOM from 'react-dom/client';
// Bundled fonts: IBM Plex Sans for the UI chrome, Lilex for the editor and code.
import '@fontsource/lilex/300.css';
import '@fontsource/lilex/400.css';
import '@fontsource/lilex/500.css';
import '@fontsource/lilex/600.css';
import '@fontsource/lilex/700.css';
import '@fontsource/ibm-plex-sans/400.css';
import '@fontsource/ibm-plex-sans/500.css';
import '@fontsource/ibm-plex-sans/600.css';
import '@fontsource/ibm-plex-sans/700.css';
import '@fontsource/ibm-plex-mono/400.css';
import '@fontsource/ibm-plex-mono/500.css';
import '@fontsource/ibm-plex-mono/600.css';
import '@fontsource/ibm-plex-mono/700.css';
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
import { initPlatform } from './shared/lib/platform';
import { AGENT_WINDOW_LABEL } from './shared/lib/agentWindow';

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
