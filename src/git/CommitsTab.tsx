import { useEffect, useMemo, useRef, useState } from 'react';
import { GitCommit, Search, X } from 'lucide-react';
import type { CommitEntry } from '../shared/ipc/ipc';
import { Highlighted, matchRanges } from '../shared/lib/match';

/** The commit log: a local fuzzy filter over the loaded commits, and another page at the end. */
export function CommitsTab({
  commits, hasMore, onLoadMore, onSelect,
}: {
  commits: CommitEntry[];
  hasMore: boolean;
  onLoadMore: () => void;
  onSelect: (c: CommitEntry) => void;
}) {
  const [query, setQuery] = useState('');
  const endRef = useRef<HTMLDivElement | null>(null);

  const matches = useMemo(() => {
    if (!query.trim()) return commits.map((c) => ({ c, ranges: [] as [number, number][] | null }));
    return commits
      .map((c) => {
        // Highlight only the message; an author or sha hit still keeps the row.
        const ranges = matchRanges(query, c.message);
        const other = matchRanges(query, c.author) ?? matchRanges(query, c.short_sha);
        return ranges || other ? { c, ranges } : null;
      })
      .filter((x): x is { c: CommitEntry; ranges: [number, number][] | null } => x !== null);
  }, [commits, query]);

  // Reaching the end of the list requests the next page.
  useEffect(() => {
    const el = endRef.current;
    if (!el || !hasMore) return;
    const io = new IntersectionObserver(
      (entries) => { if (entries.some((e) => e.isIntersecting)) onLoadMore(); },
      { threshold: 0.1 },
    );
    io.observe(el);
    return () => io.disconnect();
  }, [hasMore, onLoadMore, commits.length]);

  // Commits from the first base commit down are upstream history; the divider is drawn only unfiltered.
  const filtering = query.trim().length > 0;
  const firstBaseSha = filtering ? null : commits.find((c) => c.is_base)?.sha;

  return (
    <div className="commits-tab">
      <div className="commits-search">
        <Search size={12} strokeWidth={2} className="commits-search-icon" />
        <input
          className="commits-search-input"
          placeholder="Filter commits…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Escape') { e.stopPropagation(); setQuery(''); }
          }}
        />
        {query && (
          <button className="commits-search-clear" onClick={() => setQuery('')} title="Clear">
            <X size={11} strokeWidth={2.25} />
          </button>
        )}
      </div>

      {commits.length === 0 ? (
        <div className="sidebar-empty">No commits</div>
      ) : matches.length === 0 ? (
        <div className="sidebar-empty">
          No match in the {commits.length} commits loaded.
          {hasMore && <button className="home-link" onClick={onLoadMore}>load more</button>}
        </div>
      ) : (
        <div className="commits-list">
          {matches.map(({ c, ranges }) => (
            <div key={c.sha}>
              {c.sha === firstBaseSha && commits[0]?.sha !== c.sha && (
                <div className="commit-base-divider">
                  <span>base</span>
                </div>
              )}
              <button
                className={`commit-item ${c.is_base ? 'base' : ''}`}
                onClick={() => onSelect(c)}
                title={`${c.message} — view this commit's changes`}
              >
                <span className="commit-msg"><Highlighted text={c.message} ranges={ranges} /></span>
                <span className="commit-meta">
                  <GitCommit size={11} strokeWidth={1.75} className="commit-icon" />
                  <span className="commit-sha">{c.short_sha}</span>
                  <span className="commit-author">{c.author}</span>
                </span>
              </button>
            </div>
          ))}
          {/* The end-of-list sentinel. */}
          <div ref={endRef} className="commits-end">
            {hasMore ? `${commits.length} loaded — more…` : `${commits.length} commits`}
          </div>
        </div>
      )}
    </div>
  );
}
