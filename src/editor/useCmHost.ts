import { useEffect, useRef } from 'react';
import { EditorView } from '@codemirror/view';
import { EditorState, Compartment, type Extension } from '@codemirror/state';
import { useStore } from '../shared/store';

/** Mounts one CodeMirror view, toggles vim in place and takes focus on request. */
export function useCmHost({
  containerRef, viewRef, doc, extensions, vimExt, measureOnMount, canFocus = true, focusSignal,
  isPreview, onDestroy,
}: {
  containerRef: React.RefObject<HTMLDivElement>;
  viewRef: React.MutableRefObject<EditorView | null>;
  doc: string;
  /** A new identity remounts the view: memoize it. */
  extensions: Extension[];
  /** The vim-dependent extensions. Must be stable across renders. */
  vimExt: (on: boolean) => Extension;
  measureOnMount?: boolean;
  canFocus?: boolean;
  focusSignal?: number;
  isPreview?: boolean;
  onDestroy?: () => void;
}) {
  const vimMode = useStore((s) => s.vimMode);
  // Reconfigured in place; recreating the view would drop the buffer, selection and scroll.
  const vimCompartment = useRef(new Compartment());
  const destroyRef = useRef(onDestroy);
  destroyRef.current = onDestroy;

  // Mount the editor; remount on doc or extension change.
  useEffect(() => {
    if (!containerRef.current) return;
    const view = new EditorView({
      state: EditorState.create({
        doc,
        extensions: [vimCompartment.current.of(vimExt(useStore.getState().vimMode)), ...extensions],
      }),
      parent: containerRef.current,
    });
    viewRef.current = view;
    // The container can still have no height at the first measure.
    if (measureOnMount) view.requestMeasure();
    return () => {
      view.destroy();
      viewRef.current = null;
      destroyRef.current?.();
    };
  }, [doc, extensions, vimExt, measureOnMount, containerRef, viewRef]);

  // Toggle vim in place.
  useEffect(() => {
    viewRef.current?.dispatch({ effects: vimCompartment.current.reconfigure(vimExt(vimMode)) });
  }, [vimMode, vimExt, viewRef]);

  // Focus on a real open.
  useEffect(() => {
    if (focusSignal === undefined || isPreview || !canFocus) return;
    viewRef.current?.focus();
  }, [focusSignal, isPreview, canFocus, viewRef]);

  return { vimCompartment: vimCompartment.current };
}
