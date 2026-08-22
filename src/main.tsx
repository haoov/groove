import ReactDOM from 'react-dom/client';
// Bundle the type system locally so the app renders identically regardless of
// installed system fonts and with no remote font dependency:
//   IBM Plex Sans → UI chrome, Lilex (IBM Plex Mono fallback) → editor / code.
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
// Feature-owned stylesheets (the multi-owner files — home, sidebar, console,
// overlays — still load from global.css until their split). Order preserved
// from the old monolith; the cascade audit found no order dependence.
import './app/layout.css';
import './home/home.css';
import './agent/console.css';
import './workspace/sidebar.css';
import './files/files.css';
import './git/git.css';
import './notes/notes.css';
import './notifications/notifications.css';
import './setup/setup.css';
import './command/command.css';
import './approvals/approvals.css';
import './overview/explorer.css';
import './git/diff.css';
import './editor/editor.css';
import './workspace/workspace.css';
import './agent/agent.css';
import './notifications/feed.css';
import './overview/overview.css';
import './setup/firstrun.css';
import App from './app/App';

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <App />
);
