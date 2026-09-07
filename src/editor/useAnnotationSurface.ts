import { useMemo } from 'react';
import type { EditorView } from '@codemirror/view';
import type { Annotation, Mr, MrThread } from '../shared/ipc/ipc';
import type { AnnCtx, LineRange } from './useAnnotations';
import { deriveAnnotationSets, type AnnotationSets } from './cm/annotationSets';
import { useAnnotationPortals } from './cm/annotationPortals';

/** The dynamic state both editor hosts push into CodeMirror. */
export interface Dyn extends AnnotationSets {
  sel: LineRange | null;
  dragRange?: LineRange | null;
  fileAnnotations: Annotation[];
  /** End line of the selection in this file: where the comment form goes. */
  anchorLine: number | null;
  /** Portal target for the comment form. */
  formEl: HTMLDivElement | null;
  /** Portal target per annotated end-line for the always-visible notes. */
  annContainers: Map<number, HTMLDivElement>;
}

/** The annotation state both editor hosts share: line sets, the form anchor and the portals. */
export function useAnnotationSurface({
  ann, sel, annotations, threads, mr, repoId, filePath, formClass,
}: {
  ann: AnnCtx;
  sel: LineRange | null;
  annotations: Annotation[];
  threads: MrThread[];
  mr: Mr | null;
  repoId: string;
  filePath: string;
  formClass: string;
}) {
  const sets = useMemo(
    () => deriveAnnotationSets(annotations, threads, filePath),
    [annotations, threads, filePath],
  );

  // The selection in this file.
  const thisFileSel = sel?.repoId === repoId && sel?.filePath === filePath ? sel : null;
  const anchorLine = thisFileSel?.endLine ?? null;

  const { groups, containersRef, formRef, formEl } = useAnnotationPortals(annotations, anchorLine, formClass);

  const portalProps = {
    groups, containers: containersRef.current, formEl, sel: thisFileSel,
    annotations, threads, mr, ann, repoId, filePath,
  };

  return { sets, thisFileSel, anchorLine, groups, containersRef, formRef, formEl, portalProps };
}

/** Extends an in-flight gutter drag to the line under the pointer. */
export function extendDragAt(
  view: EditorView | null,
  e: React.MouseEvent,
  ann: AnnCtx,
  repoId: string,
  filePath: string,
  fileLineAt: (cmLine: number) => number,
) {
  if (!view) return;
  // On the wrapper, not the view: CM's domEventHandlers do not cover the gutter.
  const pos = view.posAtCoords({ x: e.clientX, y: e.clientY }, false);
  if (pos === null) return;
  const line = fileLineAt(view.state.doc.lineAt(pos).number);
  if (line > 0) ann.extendDrag(repoId, filePath, line);
}
