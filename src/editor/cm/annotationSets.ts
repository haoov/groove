import type { Annotation, MrThread } from '../../shared/ipc/ipc';

/** Gutter-indicator + highlight line-number sets shared by the code and diff
 *  editors. All sets are keyed by new-side file line number. */
export interface AnnotationSets {
  annStartNums: Set<number>;
  annotatedLineNums: Set<number>;
  threadNums: Set<number>;
  unresolvedThreadNums: Set<number>;
}

/**
 * Derive the annotated/threaded line sets for one file from its open annotations
 * and MR threads. The code editor keys these off real document line numbers and
 * the diff editor off new-side file line numbers (via its lineMap) — in both
 * cases the numbers are new-side file lines, so the derivation is identical.
 */
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
    const pos = d.notes?.[0]?.position;
    if (pos?.new_path !== filePath) continue;
    const n: number | undefined = pos.new_line ?? pos.line_range?.end?.new_line;
    if (!n) continue;
    threadNums.add(n);
    if (d.notes?.some((note) => note.resolved !== true)) unresolvedThreadNums.add(n);
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
    const pos = d.notes?.[0]?.position;
    return pos?.new_path === filePath
      && (pos.new_line === startLine || pos.line_range?.end?.new_line === startLine);
  });
}
