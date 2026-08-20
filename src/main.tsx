import ReactDOM from 'react-dom/client';
// The type system is bundled locally so the app renders identically regardless of
// installed system fonts: IBM Plex Sans for chrome, IBM Plex Mono standing in for
// Lilex (the editor/data face) until Lilex is bundled.
import '@fontsource/ibm-plex-sans/400.css';
import '@fontsource/ibm-plex-sans/500.css';
import '@fontsource/ibm-plex-sans/600.css';
import '@fontsource/ibm-plex-mono/400.css';
import '@fontsource/ibm-plex-mono/500.css';
import '@fontsource/ibm-plex-mono/600.css';
import './shared/styles/base.css';
import './shared/ui/ui.css';
import './app/app.css';
import App from './app/App';

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(<App />);
