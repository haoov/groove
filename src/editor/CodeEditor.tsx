import { useEffect, useRef } from 'react';
import { EditorState } from '@codemirror/state';
import { EditorView, keymap } from '@codemirror/view';
import { call } from '../shared/ipc/client';
import { useStore } from '../shared/store';
import { baseExtensions, languageFor } from './cm';

/** An editable CodeMirror buffer for one file tab. Cmd/Ctrl-S saves via
 *  `save_file`; edits mark the tab dirty (a dot on the tab). */
export function CodeEditor({
  sessionId, tabId, path, worktreePath, initial,
}: {
  sessionId: string; tabId: string; path: string; worktreePath: string; initial: string;
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

    const view = new EditorView({
      parent: host.current,
      state: EditorState.create({
        doc: initial,
        extensions: [
          keymap.of([{ key: 'Mod-s', preventDefault: true, run: save }]),
          ...baseExtensions(),
          ...languageFor(path),
          EditorView.updateListener.of((u) => {
            if (u.docChanged) patchTab(sessionId, tabId, { dirty: true });
          }),
        ],
      }),
    });
    viewRef.current = view;
    return () => { view.destroy(); viewRef.current = null; };
    // Mount once per tab: the buffer owns its content after open.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return <div className="cm-host" ref={host} />;
}
