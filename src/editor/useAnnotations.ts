import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useStore, useSession } from '../shared/store';
import type { Annotation } from '../shared/ipc/ipc';

/** An annotation target: a contiguous range of new-side lines in one file. */
export interface LineRange { repoId: string; filePath: string; startLine: number; endLine: number }

/** Shared annotation state + handlers threaded down to each diff file. */
export interface AnnCtx {
  sel: LineRange | null;
  dragRange: LineRange | null;
  annotationText: string;
  setAnnotationText: (s: string) => void;
  beginDrag: (repoId: string, filePath: string, line: number, e: React.MouseEvent) => void;
  extendDrag: (repoId: string, filePath: string, line: number) => void;
  selectSingle: (repoId: string, filePath: string, line: number) => void;
  submit: () => void;
  cancel: () => void;
  replyTexts: Record<string, string>;
  setReplyTexts: React.Dispatch<React.SetStateAction<Record<string, string>>>;
  /** Reply keys with an in-flight submission (drives the reply input's disabled state). */
  replyPending: Record<string, boolean>;
  submitReply: (mrId: string, threadId: string, body: string, replyKey: string) => void;
  /** Annotation ids with an in-flight "Post to MR". */
  postPending: Record<string, boolean>;
  /** Promote a local annotation to a positioned MR discussion, then resolve it. */
  postToMr: (a: Annotation, mrId: string) => void;
  /** Annotation ids with an in-flight delete. */
  deletePending: Record<string, boolean>;
  /** Delete a note outright (resolve keeps it; this removes it). */
  deleteAnnotation: (id: string) => void;
  openInEditor: (repoId: string, filePath: string, lineNum?: number) => void;
  inputRef: React.RefObject<HTMLTextAreaElement>;
}

/**
 * Owns the diff annotation selection + reply state and the handlers a diff file
 * needs (gutter drag, submit, reply). A single instance is shared across every
 * pane/tab: selections carry a repoId+filePath so a file only paints its own.
 *
 * `openInEditor` is injected so the caller decides what "open in editor" does
 * (e.g. open an edit tab in the workspace).
 */
export function useAnnotations(
  openInEditor: (repoId: string, filePath: string, lineNum?: number) => void,
): { ann: AnnCtx; sel: LineRange | null; dragRange: LineRange | null } {
  const activeTask = useSession((s) => s.activeTask);
  const addAnnotation = useSession((s) => s.addAnnotation);
  const setActiveRepoId = useSession((s) => s.setActiveRepoId);
  const resolveAnnotation = useSession((s) => s.resolveAnnotation);
  const removeAnnotation = useSession((s) => s.removeAnnotation);
  const bumpMrs = useSession((s) => s.bumpMrs);
  const setLastError = useStore((s) => s.setLastError);
  const notify = useStore((s) => s.notify);

  const [sel, setSel] = useState<LineRange | null>(null);
  const [dragRange, setDragRange] = useState<LineRange | null>(null);
  const dragRef = useRef<{ repoId: string; filePath: string; anchor: number; head: number } | null>(null);
  const [annotationText, setAnnotationText] = useState('');
  const [replyTexts, setReplyTexts] = useState<Record<string, string>>({});
  const inputRef = useRef<HTMLTextAreaElement>(null);
  // In-flight guards: refs give a synchronous guard against a double Enter/Ctrl+Enter
  // firing before React re-renders; the reply state additionally disables the UI.
  const submittingRef = useRef(false);
  const replyInFlight = useRef<Set<string>>(new Set());
  const [replyPending, setReplyPending] = useState<Record<string, boolean>>({});
  const postInFlight = useRef<Set<string>>(new Set());
  const [postPending, setPostPending] = useState<Record<string, boolean>>({});
  const deleteInFlight = useRef<Set<string>>(new Set());
  const [deletePending, setDeletePending] = useState<Record<string, boolean>>({});

  // Focus the comment input when a range is selected.
  useEffect(() => {
    if (sel) setTimeout(() => inputRef.current?.focus(), 50);
  }, [sel]);

  // Finalize a gutter drag on mouse release anywhere.
  useEffect(() => {
    const onUp = () => {
      const d = dragRef.current;
      if (!d) return;
      dragRef.current = null;
      setDragRange(null);
      setSel({ repoId: d.repoId, filePath: d.filePath, startLine: Math.min(d.anchor, d.head), endLine: Math.max(d.anchor, d.head) });
      setAnnotationText('');
    };
    window.addEventListener('mouseup', onUp);
    return () => window.removeEventListener('mouseup', onUp);
  }, []);

  // Esc clears the active selection.
  useEffect(() => {
    if (!sel) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') { setSel(null); setAnnotationText(''); }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [sel]);

  const submit = async () => {
    if (submittingRef.current) return;
    if (!sel || !annotationText.trim() || !activeTask) return;
    submittingRef.current = true;
    try {
      const created = await invoke<Annotation>('create_annotation', {
        sessionId: activeTask.short_id,
        repoId: sel.repoId,
        filePath: sel.filePath,
        startLine: sel.startLine,
        endLine: sel.endLine,
        content: annotationText.trim(),
        author: 'human',
      });
      addAnnotation(created);
      setAnnotationText('');
      setSel(null);
    } catch (e) {
      setLastError(String(e));
    } finally {
      submittingRef.current = false;
    }
  };

  const submitReply = async (mrId: string, threadId: string, body: string, replyKey: string) => {
    if (!body.trim() || replyInFlight.current.has(replyKey)) return;
    replyInFlight.current.add(replyKey);
    setReplyPending((p) => ({ ...p, [replyKey]: true }));
    try {
      await invoke('reply_to_thread', { mrId, threadId, body: body.trim() });
      setReplyTexts((prev) => { const n = { ...prev }; delete n[replyKey]; return n; });
    } catch (e) {
      setLastError(String(e));
    } finally {
      replyInFlight.current.delete(replyKey);
      setReplyPending((p) => { const n = { ...p }; delete n[replyKey]; return n; });
    }
  };

  // Publish a drafted annotation as a positioned MR discussion (anchored at its
  // start line on the MR head), then resolve it locally — draft locally, post
  // deliberately. Caveat: positions reference the REMOTE head, so post before
  // making local commits in a review worktree.
  const postToMr = async (a: Annotation, mrId: string) => {
    if (postInFlight.current.has(a.id)) return;
    postInFlight.current.add(a.id);
    setPostPending((p) => ({ ...p, [a.id]: true }));
    try {
      await invoke('post_mr_comment', {
        mrId,
        body: a.content,
        filePath: a.file_path,
        line: a.start_line,
      });
      await invoke('resolve_annotation', { id: a.id });
      resolveAnnotation(a.id);
      bumpMrs(); // the new thread shows up in Discussion + the diff gutter
      notify({ kind: 'success', source: 'mr', taskId: a.session_id, title: `Comment posted on ${a.file_path.split('/').pop()}:${a.start_line}` });
    } catch (e) {
      setLastError(String(e));
    } finally {
      postInFlight.current.delete(a.id);
      setPostPending((p) => { const n = { ...p }; delete n[a.id]; return n; });
    }
  };

  const deleteAnnotation = async (id: string) => {
    if (deleteInFlight.current.has(id)) return;
    deleteInFlight.current.add(id);
    setDeletePending((p) => ({ ...p, [id]: true }));
    try {
      await invoke('delete_annotation', { id });
      removeAnnotation(id);
    } catch (e) {
      setLastError(String(e));
    } finally {
      deleteInFlight.current.delete(id);
      setDeletePending((p) => { const n = { ...p }; delete n[id]; return n; });
    }
  };

  const beginDrag = (repoId: string, filePath: string, line: number, e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setActiveRepoId(repoId);
    dragRef.current = { repoId, filePath, anchor: line, head: line };
    setDragRange({ repoId, filePath, startLine: line, endLine: line });
  };

  const extendDrag = (repoId: string, filePath: string, line: number) => {
    const d = dragRef.current;
    if (!d || d.repoId !== repoId || d.filePath !== filePath) return;
    d.head = line;
    setDragRange({ repoId, filePath, startLine: Math.min(d.anchor, line), endLine: Math.max(d.anchor, line) });
  };

  const selectSingle = (repoId: string, filePath: string, line: number) => {
    setActiveRepoId(repoId);
    setSel((prev) =>
      prev && prev.repoId === repoId && prev.filePath === filePath && prev.startLine === line && prev.endLine === line
        ? null
        : { repoId, filePath, startLine: line, endLine: line }
    );
    setAnnotationText('');
  };

  const ann: AnnCtx = {
    sel, dragRange, annotationText, setAnnotationText,
    beginDrag, extendDrag, selectSingle,
    submit, cancel: () => { setSel(null); setAnnotationText(''); },
    replyTexts, setReplyTexts, replyPending, submitReply,
    postPending, postToMr, deletePending, deleteAnnotation, openInEditor, inputRef,
  };

  return { ann, sel, dragRange };
}
