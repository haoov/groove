// Recursive pane layout — a binary tree of splits over pane ids. Pure helpers;
// the pane registry itself (tabs etc.) lives beside it in the session store.

export type SplitDir = 'row' | 'col'; // row = children side-by-side, col = stacked

export type LayoutNode =
  | { kind: 'leaf'; paneId: string }
  | { kind: 'split'; id: string; dir: SplitDir; ratio: number; a: LayoutNode; b: LayoutNode };

let splitSeq = 0;
const newSplitId = () => `split-${++splitSeq}`;

export const leaf = (paneId: string): LayoutNode => ({ kind: 'leaf', paneId });

/** Wrap the WHOLE tree in a split with a new leaf as `b` — the new pane spans
 *  the full width (col) / full height (row), dock-style. */
export function splitRoot(node: LayoutNode, dir: SplitDir, newPaneId: string, ratio = 0.5): LayoutNode {
  return { kind: 'split', id: newSplitId(), dir, ratio, a: node, b: leaf(newPaneId) };
}

/** Replace the leaf for `paneId` with a split of it and a new leaf for `newPaneId`
 *  (new pane goes to the right / below). Returns the same tree if not found. */
export function splitLeaf(node: LayoutNode, paneId: string, dir: SplitDir, newPaneId: string, ratio = 0.5): LayoutNode {
  if (node.kind === 'leaf') {
    if (node.paneId !== paneId) return node;
    return { kind: 'split', id: newSplitId(), dir, ratio, a: node, b: leaf(newPaneId) };
  }
  const a = splitLeaf(node.a, paneId, dir, newPaneId, ratio);
  if (a !== node.a) return { ...node, a };
  const b = splitLeaf(node.b, paneId, dir, newPaneId, ratio);
  if (b !== node.b) return { ...node, b };
  return node;
}

/** Remove the leaf for `paneId`, collapsing its parent split into the sibling.
 *  Returns [newTree, nearestSurvivingPaneId]. Removing the only leaf is a no-op. */
export function removeLeaf(node: LayoutNode, paneId: string): [LayoutNode, string | null] {
  if (node.kind === 'leaf') return [node, null]; // root leaf — never removed
  const inA = containsLeaf(node.a, paneId);
  const inB = containsLeaf(node.b, paneId);
  if (!inA && !inB) return [node, null];

  const target = inA ? node.a : node.b;
  const sibling = inA ? node.b : node.a;
  if (target.kind === 'leaf') {
    // Direct child: the split collapses into the sibling; focus its first leaf.
    return [sibling, firstLeaf(sibling)];
  }
  const [replaced, survivor] = removeLeaf(target, paneId);
  const next = inA ? { ...node, a: replaced } : { ...node, b: replaced };
  return [next, survivor];
}

export function containsLeaf(node: LayoutNode, paneId: string): boolean {
  if (node.kind === 'leaf') return node.paneId === paneId;
  return containsLeaf(node.a, paneId) || containsLeaf(node.b, paneId);
}

export function firstLeaf(node: LayoutNode): string {
  return node.kind === 'leaf' ? node.paneId : firstLeaf(node.a);
}

/** Pane ids in visual traversal order (left→right, top→bottom). */
export function leafOrder(node: LayoutNode): string[] {
  if (node.kind === 'leaf') return [node.paneId];
  return [...leafOrder(node.a), ...leafOrder(node.b)];
}

/** Update one split's ratio (clamped so neither side collapses). */
export function setRatio(node: LayoutNode, splitId: string, ratio: number): LayoutNode {
  if (node.kind === 'leaf') return node;
  if (node.id === splitId) return { ...node, ratio: Math.min(0.85, Math.max(0.15, ratio)) };
  const a = setRatio(node.a, splitId, ratio);
  if (a !== node.a) return { ...node, a };
  const b = setRatio(node.b, splitId, ratio);
  if (b !== node.b) return { ...node, b };
  return node;
}
