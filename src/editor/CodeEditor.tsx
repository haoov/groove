import { useEffect, useRef } from 'react';
import { EditorState } from '@codemirror/state';
import { EditorView, keymap } from '@codemirror/view';
import { call } from '../shared/ipc/client';
import { on, EV } from '../shared/ipc/events';
import { useStore } from '../shared/store';
import { baseExtensions, languageFor } from './cm';
import { annotationExtension, setAnnotations, type AnnHandlers } from './annotations';
import type { Annotation } from '../shared/ipc/generated';

/** An editable CodeMirror buffer for one file tab. Cmd/Ctrl-S saves via
 *  `save_file`; edits mark the tab dirty. Inline annotations (this session's,
 *  for this file) render under their lines; Mod-Shift-a adds one on the
 *  selection. */
export function CodeEditor({
  sessionId, tabId, repoId, path, worktreePath, initial,
}: {
  sessionId: string; tabId: string; repoId: string; path: string; worktreePath: string; initial: string;
}) {
  const host = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const patchTab = useStore((s) => s.patchTab);

  useEffect(() => {
    if (!host.current) return;

    const save = (view: EditorView) => {
      const content = view.state.doc.toString();
      call('save_file', { worktreePath, filePath: path, content })
        .then(() => patchTab(sessionId, tabId, { dirty: false }))
        .catch((e) => console.warn('save_file failed', e));
      return true;
    };

    const refetch = () => {
      call<Annotation[]>('get_annotations', { sessionId, repoId })
        .then((all) => viewRef.current?.dispatch({ effects: setAnnotations.of(all.filter((a) => a.file_path === path)) }))
        .catch((e) => console.warn('get_annotations failed', e));
    };

    const handlers: AnnHandlers = {
      resolve: (id) => { call('resolve_annotation', { id }).then(refetch).catch(() => {}); },
      remove: (id) => { call('delete_annotation', { id }).then(refetch).catch(() => {}); },
      create: (startLine, endLine, content) => {
        call('create_annotation', { sessionId, repoId, filePath: path, startLine, endLine, content, author: 'you' })
          .then(refetch).catch((e) => console.warn('create_annotation failed', e));
      },
    };

    const view = new EditorView({
      parent: host.current,
      state: EditorState.create({
        doc: initial,
        extensions: [
          keymap.of([{ key: 'Mod-s', preventDefault: true, run: save }]),
          ...baseExtensions(),
          ...languageFor(path),
          annotationExtension(handlers),
          EditorView.updateListener.of((u) => {
            if (u.docChanged) patchTab(sessionId, tabId, { dirty: true });
          }),
        ],
      }),
    });
    viewRef.current = view;
    refetch();

    // The agent creates/resolves annotations too; refetch when it does.
    const uns = [on(EV.annotationCreated, refetch), on(EV.annotationResolved, refetch)];

    return () => {
      uns.forEach((p) => p.then((u) => u()));
      view.destroy();
      viewRef.current = null;
    };
    // Mount once per tab: the buffer owns its content after open.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return <div className="cm-host" ref={host} />;
}
