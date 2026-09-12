import type { Annotation, MrThread } from '../../shared/ipc/ipc';

/** Line-number sets for gutter indicators and highlights, keyed by new-side file line. */
export interface AnnotationSets {
  annStartNums: Set<number>;
  annotatedLineNums: Set<number>;
  threadNums: Set<number>;
  unresolvedThreadNums: Set<number>;
}

/** Derives the annotated and threaded line sets for one file. */
export function deriveAnnotationSets(
  annotations: Annotation[],
  threads: MrThread[],
  filePath: string,
): AnnotationSets {
  const annStartNums = new Set<number>();
  const annotatedLineNums = new Set<number>();
  for (const a of annotations) {
    annStartNums.add(a.start_line);
    for (let n = a.start_line; n <= a.end_line; n++) annotatedLineNums.add(n);
  }
  const threadNums = new Set<number>();
  const unresolvedThreadNums = new Set<number>();
  for (const d of threads) {
    const pos = d.notes[0]?.position;
    if (pos?.new_path !== filePath) continue;
    const n = pos.new_line ?? pos.end_new_line;
    if (!n) continue;
    threadNums.add(n);
    if (d.notes.some((note) => !note.resolved)) unresolvedThreadNums.add(n);
  }
  return { annStartNums, annotatedLineNums, threadNums, unresolvedThreadNums };
}

/** Annotations whose range starts on `startLine` (the inline panel's anchor). */
export function annotationsForStartLine(annotations: Annotation[], startLine: number): Annotation[] {
  return annotations.filter((a) => a.start_line === startLine);
}

/** MR threads positioned at `startLine` of `filePath`. */
export function threadsForStartLine(threads: MrThread[], filePath: string, startLine: number): MrThread[] {
  return threads.filter((d) => {
    const pos = d.notes[0]?.position;
    return pos?.new_path === filePath
      && (pos.new_line === startLine || pos.end_new_line === startLine);
  });
}
