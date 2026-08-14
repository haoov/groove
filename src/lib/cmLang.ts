import { LanguageSupport } from '@codemirror/language';
import { javascript } from '@codemirror/lang-javascript';
import { python } from '@codemirror/lang-python';
import { rust } from '@codemirror/lang-rust';
import { java } from '@codemirror/lang-java';
import { cpp } from '@codemirror/lang-cpp';
import { css } from '@codemirror/lang-css';
import { json } from '@codemirror/lang-json';
import { xml } from '@codemirror/lang-xml';
import { markdown } from '@codemirror/lang-markdown';
import { sql } from '@codemirror/lang-sql';
import { yaml } from '@codemirror/lang-yaml';
import { gotmpl } from './cm/gotmpl';

export function cmLangFor(lang: string): LanguageSupport | null {
  switch (lang) {
    case 'javascript':
    case 'jsx':
      return javascript({ jsx: true });
    case 'typescript':
    case 'tsx':
      return javascript({ jsx: true, typescript: true });
    case 'python': return python();
    case 'rust':    return rust();
    case 'java':    return java();
    case 'c':
    case 'cpp':
    case 'csharp':  return cpp();
    case 'css':
    case 'scss':    return css();
    case 'json':    return json();
    case 'html':
    case 'xml':     return xml();
    case 'markdown': return markdown();
    case 'sql':     return sql();
    case 'yaml':    return yaml();
    case 'gotmpl':  return gotmpl();
    default:        return null;
  }
}
