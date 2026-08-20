import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

/** Read-only markdown, GitHub-flavored. Styling lives in ui.css (`.md`). */
export function Markdown({ children }: { children: string }) {
  return (
    <div className="md">
      <ReactMarkdown remarkPlugins={[remarkGfm]}>{children}</ReactMarkdown>
    </div>
  );
}
