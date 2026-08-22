import { useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { Eye, Loader2, Pencil, X } from 'lucide-react';
import { useStore } from '../shared/store';
import { Markdown } from '../shared/ui/Markdown';

/**
 * The task body: rendered markdown, or the raw markdown to edit by hand.
 *
 * Saving does NOT write directly. It queues a confirmation (op `notion.body`)
 * because replacing a page's children can destroy blocks markdown cannot express
 * — the modal is where you see what is about to change, and the backend refuses
 * outright if the page holds anything unrepresentable.
 */
export function BodyEditor({
  taskId, notionPageId, markdown, loading, onSaved,
}: {
  taskId: string;
  notionPageId: string;
  markdown: string;
  loading: boolean;
  onSaved: () => void;
}) {
  const notify = useStore((s) => s.notify);
  const setLastError = useStore((s) => s.setLastError);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState('');
  const [saving, setSaving] = useState(false);

  const startEditing = () => {
    setDraft(markdown);
    setEditing(true);
  };

  const save = async () => {
    setSaving(true);
    try {
      await invoke('request_task_body_update', {
        notionPageId, taskId, markdown: draft, force: false,
      });
      notify({
        kind: 'info',
        source: 'notion',
        taskId,
        title: 'Body update queued for approval',
        detail: 'Review the change, then approve it to write to Notion.',
      });
      setEditing(false);
      onSaved();
    } catch (e) {
      setLastError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const dirty = editing && draft !== markdown;

  return (
    <section className="overview-section">
      <h3 className="overview-section-title">
        Description
        <span className="overview-section-head-actions">
          {editing ? (
            <>
              <button className="home-link" disabled={saving || !dirty} onClick={save}>
                {saving ? <Loader2 size={11} className="spin" /> : null}
                save to Notion
              </button>
              <button className="home-link" onClick={() => setEditing(false)}>
                <X size={11} strokeWidth={2} />
                cancel
              </button>
            </>
          ) : (
            <button className="home-link" onClick={startEditing} disabled={loading}>
              <Pencil size={11} strokeWidth={2} />
              edit
            </button>
          )}
        </span>
      </h3>

      {editing ? (
        <>
          <textarea
            className="body-editor"
            autoFocus
            spellCheck={false}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
          />
          <p className="body-editor-hint">
            <Eye size={10} strokeWidth={2} />
            Markdown. Saving replaces the page body — anything Notion holds that
            markdown can&rsquo;t express (images, embeds, sub-databases) would be
            lost, so the save is refused if the page has any.
          </p>
        </>
      ) : loading ? (
        <p className="overview-empty-body">Loading…</p>
      ) : markdown.trim() ? (
        <Markdown text={markdown} />
      ) : (
        <p className="overview-empty-body">No description.</p>
      )}
    </section>
  );
}
