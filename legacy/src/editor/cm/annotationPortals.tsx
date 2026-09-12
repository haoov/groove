import { useEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { AnnotationBlock, InlineAnnotations } from '../AnnotationBlock';
import { annotationsForStartLine, threadsForStartLine } from './annotationSets';
import type { Annotation, Mr, MrThread } from '../../shared/ipc/ipc';
import type { AnnCtx, LineRange } from '../useAnnotations';

/**
 * The annotation surface both editors show: a note block under every annotated
 * line, plus the comment form at the selection, portalled into CodeMirror widgets.
 */

/** One stable container element per annotated end-line, plus one for the inline form.
 *  A new element per render makes CodeMirror re-measure and drop the portal's DOM. */
export function useAnnotationPortals(
  annotations: Annotation[],
  anchorLine: number | null,
  formClass: string,
) {
  const containersRef = useRef<Map<number, HTMLDivElement>>(new Map());
  const formRef = useRef<HTMLDivElement | null>(null);
  const [formEl, setFormEl] = useState<HTMLDivElement | null>(null);

  const groups = useMemo(() => {
    const byEnd = new Map<number, Annotation[]>();
    for (const a of annotations) {
      const group = byEnd.get(a.end_line);
      if (group) group.push(a);
      else byEnd.set(a.end_line, [a]);
    }
    const containers = containersRef.current;
    for (const line of [...containers.keys()]) {
      if (!byEnd.has(line)) containers.delete(line);
    }
    for (const line of byEnd.keys()) {
      if (!containers.has(line)) {
        const el = document.createElement('div');
        el.className = 'diff-inline-portal';
        containers.set(line, el);
      }
    }
    return byEnd;
  }, [annotations]);

  useEffect(() => {
    if (anchorLine !== null) {
      const el = document.createElement('div');
      el.className = formClass;
      formRef.current = el;
      setFormEl(el);
    } else {
      formRef.current = null;
      setFormEl(null);
    }
  }, [anchorLine, formClass]);

  return { groups, containersRef, formRef, formEl };
}

/** Portals the annotation blocks into the containers the hook created. */
export function AnnotationPortals({
  groups, containers, formEl, sel, annotations, threads, mr, ann, repoId, filePath,
}: {
  groups: Map<number, Annotation[]>;
  containers: Map<number, HTMLDivElement>;
  formEl: HTMLDivElement | null;
  /** The selection in THIS file, or null. */
  sel: LineRange | null;
  annotations: Annotation[];
  threads: MrThread[];
  mr: Mr | null;
  ann: AnnCtx;
  repoId: string;
  filePath: string;
}) {
  return (
    <>
      {[...groups.entries()].map(([line, anns]) => {
        const el = containers.get(line);
        return el ? createPortal(<InlineAnnotations anns={anns} ann={ann} mr={mr} />, el, `anns-${line}`) : null;
      })}
      {formEl && sel && createPortal(
        <AnnotationBlock
          range={sel}
          lineAnnotations={annotationsForStartLine(annotations, sel.startLine)}
          lineThreads={threadsForStartLine(threads, filePath, sel.startLine)}
          mr={mr}
          ann={ann}
          repoId={repoId}
          filePath={filePath}
          lineNum={sel.startLine}
        />,
        formEl,
      )}
    </>
  );
}
