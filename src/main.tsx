import ReactDOM from 'react-dom/client';
// Bundle the type system locally so the app renders identically regardless of
// installed system fonts and with no remote font dependency:
//   IBM Plex Sans → UI chrome, Lilex (IBM Plex Mono fallback) → editor / code.
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
import App from './app/App';

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <App />
);
