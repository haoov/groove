import { ChevronDown, ChevronRight } from 'lucide-react';
import { matchRanges, Highlighted } from '../shared/lib/match';
import { FileTypeIcon } from './tree';
import { rowLine, type FileSearch } from './useFileSearch';

function splitPath(path: string): { name: string; dir: string } {
  const idx = path.lastIndexOf('/');
  return idx === -1 ? { name: path, dir: '' } : { name: path.slice(idx + 1), dir: path.slice(0, idx) };
}

/** Grep hits, grouped by file and foldable. */
export function GrepResults({ search }: { search: FileSearch }) {
  const {
    query, rows, clampedSel, setSelectedIdx, grepFiles, grepLoading, grepCollapsed,
    setGrepCollapsed, commitGrep, resultsRef,
  } = search;

  return (
    <div className="files-list file-search-results" ref={resultsRef}>
      {query.trim().length < 2 ? (
        <div className="sidebar-empty">Type at least 2 characters</div>
      ) : grepLoading && grepFiles.length === 0 ? (
        <div className="sidebar-empty">Searching…</div>
      ) : grepFiles.length === 0 ? (
        <div className="sidebar-empty">No matches</div>
      ) : (
        rows.map((row, i) => {
          const selected = i === clampedSel ? 'nav-selected' : '';
          if (row.kind === 'file') {
            const { name, dir } = splitPath(row.file.file);
            const collapsed = grepCollapsed.has(row.file.file);
            const n = row.file.matches.length;
            return (
              <button
                key={row.file.file}
                className={`file-item grep-file ${selected}`}
                title={`${row.file.file} — ${n} match${n === 1 ? '' : 'es'}`}
                tabIndex={-1}
                onMouseEnter={() => setSelectedIdx(i)}
                onClick={(e) => {
                  // The chevron collapses; the row opens the first match.
                  if ((e.target as HTMLElement).closest('.grep-collapse')) return;
                  commitGrep(row.file.file, rowLine(row));
                }}
              >
                <span
                  className="grep-collapse"
                  role="presentation"
                  onClick={() => setGrepCollapsed((prev) => {
                    const next = new Set(prev);
                    if (next.has(row.file.file)) next.delete(row.file.file);
                    else next.add(row.file.file);
                    return next;
                  })}
                >
                  {collapsed ? <ChevronRight size={12} strokeWidth={2} /> : <ChevronDown size={12} strokeWidth={2} />}
                </span>
                <span className="file-icon"><FileTypeIcon name={name} /></span>
                <span className="file-name">{name}</span>
                {dir && <span className="file-search-dir">{dir}</span>}
                <span className="file-search-count">{n}</span>
              </button>
            );
          }
          return (
            <button
              key={`${row.file.file}:${row.match.line}`}
              className={`file-item grep-match ${selected}`}
              title={`${row.file.file}:${row.match.line}`}
              tabIndex={-1}
              onMouseEnter={() => setSelectedIdx(i)}
              onClick={() => commitGrep(row.file.file, row.match.line)}
            >
              <span className="grep-match-line">{row.match.line}</span>
              <span className="grep-match-text">
                <Highlighted text={row.match.content.trim()} ranges={matchRanges(query, row.match.content.trim())} />
              </span>
            </button>
          );
        })
      )}
    </div>
  );
}

/** Fuzzy path matches for the typed query. */
export function NameResults({ search }: { search: FileSearch }) {
  const { query, results, clampedSel, setSelectedIdx, commitPath, resultsRef } = search;

  return (
    <div className="files-list file-search-results" ref={resultsRef}>
      {results.length === 0 ? (
        <div className="sidebar-empty">No matching files</div>
      ) : (
        results.map((r, i) => {
          const { name, dir } = splitPath(r.f);
          return (
            <button
              key={r.f}
              className={`file-item ${i === clampedSel ? 'nav-selected' : ''}`}
              title={r.f}
              tabIndex={-1}
              onMouseEnter={() => setSelectedIdx(i)}
              onClick={() => commitPath(r.f)}
            >
              <span className="file-icon"><FileTypeIcon name={name} /></span>
              <span className="file-name"><Highlighted text={name} ranges={matchRanges(query, name)} /></span>
              {dir && <span className="file-search-dir">{dir}</span>}
            </button>
          );
        })
      )}
    </div>
  );
}
