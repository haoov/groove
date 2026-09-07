import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
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
  /** Reply keys with an in-flight submission. */
  replyPending: Record<string, boolean>;
  submitReply: (mrId: string, threadId: string, body: string, replyKey: string) => void;
  /** Annotation ids with an in-flight "Post to MR". */
  postPending: Record<string, boolean>;
  /** Posts a local annotation as a positioned MR discussion, then resolves it. */
  postToMr: (a: Annotation, mrId: string) => void;
  /** The annotation open for editing, and its draft body. */
  editingId: string | null;
  editText: string;
  setEditText: (s: string) => void;
  beginEdit: (a: Annotation) => void;
  cancelEdit: () => void;
  /** Saves the draft over the annotation being edited. */
  saveEdit: () => void;
  /** Annotation ids with an in-flight edit. */
  editPending: Record<string, boolean>;
  /** Annotation ids with an in-flight resolve. */
  resolvePending: Record<string, boolean>;
  /** Marks a note resolved; the record stays. */
  resolveNote: (id: string) => void;
  /** Annotation ids with an in-flight delete. */
  deletePending: Record<string, boolean>;
  /** Deletes a note outright. */
  deleteAnnotation: (id: string) => void;
  openInEditor: (repoId: string, filePath: string, lineNum?: number) => void;
  inputRef: React.RefObject<HTMLTextAreaElement>;
}

/**
 * Owns the annotation selection, reply state and handlers shared by every pane.
 * `openInEditor` is injected by the caller.
 */
export function useAnnotations(
  openInEditor: (repoId: string, filePath: string, lineNum?: number) => void,
): { ann: AnnCtx; sel: LineRange | null; dragRange: LineRange | null } {
  const activeTask = useSession((s) => s.activeTask);
  const addAnnotation = useSession((s) => s.addAnnotation);
  const updateAnnotation = useSession((s) => s.updateAnnotation);
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
  // In-flight guards: the refs block a double Enter before React re-renders.
  const submittingRef = useRef(false);
  const replyInFlight = useRef<Set<string>>(new Set());
  const [replyPending, setReplyPending] = useState<Record<string, boolean>>({});
  const postInFlight = useRef<Set<string>>(new Set());
  const [postPending, setPostPending] = useState<Record<string, boolean>>({});
  const deleteInFlight = useRef<Set<string>>(new Set());
  const [deletePending, setDeletePending] = useState<Record<string, boolean>>({});
  const resolveInFlight = useRef<Set<string>>(new Set());
  const [resolvePending, setResolvePending] = useState<Record<string, boolean>>({});
  const editInFlight = useRef<Set<string>>(new Set());
  const [editPending, setEditPending] = useState<Record<string, boolean>>({});
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editText, setEditText] = useState('');

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

  const submit = useCallback(async () => {
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
      setLastError(e);
    } finally {
      submittingRef.current = false;
    }
  }, [sel, annotationText, activeTask, addAnnotation, setLastError]);

  const submitReply = useCallback(async (mrId: string, threadId: string, body: string, replyKey: string) => {
    if (!body.trim() || replyInFlight.current.has(replyKey)) return;
    replyInFlight.current.add(replyKey);
    setReplyPending((p) => ({ ...p, [replyKey]: true }));
    try {
      await invoke('reply_to_thread', { mrId, threadId, body: body.trim() });
      setReplyTexts((prev) => { const n = { ...prev }; delete n[replyKey]; return n; });
    } catch (e) {
      setLastError(e);
    } finally {
      replyInFlight.current.delete(replyKey);
      setReplyPending((p) => { const n = { ...p }; delete n[replyKey]; return n; });
    }
  }, [setLastError]);

  // Positions reference the remote MR head: post before local commits in a review worktree.
  const postToMr = useCallback(async (a: Annotation, mrId: string) => {
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
      bumpMrs();
      notify({ kind: 'success', source: 'mr', taskId: a.session_id, title: `Comment posted on ${a.file_path.split('/').pop()}:${a.start_line}` });
    } catch (e) {
      setLastError(e);
    } finally {
      postInFlight.current.delete(a.id);
      setPostPending((p) => { const n = { ...p }; delete n[a.id]; return n; });
    }
  }, [resolveAnnotation, bumpMrs, notify, setLastError]);

  const beginEdit = useCallback((a: Annotation) => {
    setEditingId(a.id);
    setEditText(a.content);
  }, []);

  const cancelEdit = useCallback(() => {
    setEditingId(null);
    setEditText('');
  }, []);

  const saveEdit = useCallback(async () => {
    const id = editingId;
    const content = editText.trim();
    if (!id || !content || editInFlight.current.has(id)) return;
    editInFlight.current.add(id);
    setEditPending((p) => ({ ...p, [id]: true }));
    try {
      await invoke('update_annotation', { id, content });
      updateAnnotation(id, content);
      cancelEdit();
    } catch (e) {
      setLastError(e);
    } finally {
      editInFlight.current.delete(id);
      setEditPending((p) => { const n = { ...p }; delete n[id]; return n; });
    }
  }, [editingId, editText, updateAnnotation, cancelEdit, setLastError]);

  const resolveNote = useCallback(async (id: string) => {
    if (resolveInFlight.current.has(id)) return;
    resolveInFlight.current.add(id);
    setResolvePending((p) => ({ ...p, [id]: true }));
    try {
      await invoke('resolve_annotation', { id });
      resolveAnnotation(id);
    } catch (e) {
      setLastError(e);
    } finally {
      resolveInFlight.current.delete(id);
      setResolvePending((p) => { const n = { ...p }; delete n[id]; return n; });
    }
  }, [resolveAnnotation, setLastError]);

  const deleteAnnotation = useCallback(async (id: string) => {
    if (deleteInFlight.current.has(id)) return;
    deleteInFlight.current.add(id);
    setDeletePending((p) => ({ ...p, [id]: true }));
    try {
      await invoke('delete_annotation', { id });
      removeAnnotation(id);
    } catch (e) {
      setLastError(e);
    } finally {
      deleteInFlight.current.delete(id);
      setDeletePending((p) => { const n = { ...p }; delete n[id]; return n; });
    }
  }, [removeAnnotation, setLastError]);

  const beginDrag = useCallback((repoId: string, filePath: string, line: number, e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setActiveRepoId(repoId);
    dragRef.current = { repoId, filePath, anchor: line, head: line };
    setDragRange({ repoId, filePath, startLine: line, endLine: line });
  }, [setActiveRepoId]);

  const extendDrag = useCallback((repoId: string, filePath: string, line: number) => {
    const d = dragRef.current;
    if (!d || d.repoId !== repoId || d.filePath !== filePath) return;
    d.head = line;
    setDragRange({ repoId, filePath, startLine: Math.min(d.anchor, line), endLine: Math.max(d.anchor, line) });
  }, []);

  const selectSingle = useCallback((repoId: string, filePath: string, line: number) => {
    setActiveRepoId(repoId);
    setSel((prev) =>
      prev && prev.repoId === repoId && prev.filePath === filePath && prev.startLine === line && prev.endLine === line
        ? null
        : { repoId, filePath, startLine: line, endLine: line }
    );
    setAnnotationText('');
  }, [setActiveRepoId]);

  const cancel = useCallback(() => { setSel(null); setAnnotationText(''); }, []);

  const ann: AnnCtx = useMemo(() => ({
    sel, dragRange, annotationText, setAnnotationText,
    beginDrag, extendDrag, selectSingle,
    submit, cancel,
    replyTexts, setReplyTexts, replyPending, submitReply,
    postPending, postToMr, resolvePending, resolveNote,
    deletePending, deleteAnnotation, openInEditor, inputRef,
    editingId, editText, setEditText, beginEdit, cancelEdit, saveEdit, editPending,
  }), [
    sel, dragRange, annotationText,
    beginDrag, extendDrag, selectSingle,
    submit, cancel,
    replyTexts, replyPending, submitReply,
    postPending, postToMr, resolvePending, resolveNote,
    deletePending, deleteAnnotation, openInEditor,
    editingId, editText, beginEdit, cancelEdit, saveEdit, editPending,
  ]);

  return { ann, sel, dragRange };
}
