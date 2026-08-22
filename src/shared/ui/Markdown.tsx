import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { openExternal } from '../lib/openExternal';

/**
 * GFM markdown renderer for overview pages (MR descriptions etc.), styled with
 * the overview's `nb-*` typography. Links always open in the system browser —
 * never navigate the webview. Safe by construction (no raw HTML rendering).
 */
export function Markdown({ text }: { text: string }) {
  return (
    <div className="markdown-body">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          h1: ({ children }) => <h1 className="nb-h1">{children}</h1>,
          h2: ({ children }) => <h2 className="nb-h2">{children}</h2>,
          h3: ({ children }) => <h3 className="nb-h3">{children}</h3>,
          h4: ({ children }) => <h3 className="nb-h3">{children}</h3>,
          h5: ({ children }) => <h3 className="nb-h3">{children}</h3>,
          h6: ({ children }) => <h3 className="nb-h3">{children}</h3>,
          p: ({ children }) => <p className="nb-p">{children}</p>,
          ul: ({ children }) => <ul className="nb-list nb-list-ul">{children}</ul>,
          ol: ({ children }) => <ol className="nb-list nb-list-ol">{children}</ol>,
          li: ({ children }) => <li className="nb-bullet">{children}</li>,
          blockquote: ({ children }) => <blockquote className="nb-quote">{children}</blockquote>,
          hr: () => <hr className="nb-divider" />,
          a: ({ href, children }) => (
            <a
              className="nb-link"
              href={href}
              onClick={(e) => {
                e.preventDefault();
                if (href) openExternal(href);
              }}
            >
              {children}
            </a>
          ),
          // Fenced blocks arrive as <pre><code>; inline code as bare <code>.
          pre: ({ children }) => <pre className="nb-code">{children}</pre>,
          code: ({ children, className }) => <code className={className}>{children}</code>,
          table: ({ children }) => (
            <div className="nb-table-wrap">
              <table className="nb-table">{children}</table>
            </div>
          ),
          input: ({ checked }) => (
            // GFM task-list checkboxes, read-only.
            <span className="nb-todo-box" aria-hidden>{checked ? '✓' : '○'}</span>
          ),
        }}
      >
        {text}
      </ReactMarkdown>
    </div>
  );
}
