import { describe, expect, it } from 'vitest';
import {
  containsLeaf, firstLeaf, leaf, leafOrder, removeLeaf, setRatio, splitLeaf, splitRoot,
  type LayoutNode,
} from './layout';

// The pane tree is the one data structure in the app with invariants a user can
// break by clicking: every split must keep exactly two children, closing a pane
// must collapse its parent into the sibling, and no pane id may be lost or
// duplicated. These check the shape, not the rendering.

/** Every invariant a tree must satisfy after any operation. */
function assertWellFormed(node: LayoutNode) {
  const ids = leafOrder(node);
  expect(new Set(ids).size, `duplicate pane id in ${JSON.stringify(ids)}`).toBe(ids.length);
  const walk = (n: LayoutNode) => {
    if (n.kind === 'leaf') return;
    expect(n.a).toBeDefined();
    expect(n.b).toBeDefined();
    expect(n.ratio).toBeGreaterThan(0);
    expect(n.ratio).toBeLessThan(1);
    walk(n.a);
    walk(n.b);
  };
  walk(node);
}

describe('splitLeaf', () => {
  it('splits the named leaf and puts the new pane second', () => {
    const t = splitLeaf(leaf('p1'), 'p1', 'row', 'p2');
    expect(t.kind).toBe('split');
    expect(leafOrder(t)).toEqual(['p1', 'p2']);
    assertWellFormed(t);
  });

  it('leaves the tree untouched when the pane is not there', () => {
    const t = leaf('p1');
    expect(splitLeaf(t, 'nope', 'row', 'p2')).toBe(t);
  });

  it('splits a nested leaf without disturbing its siblings', () => {
    let t: LayoutNode = leaf('p1');
    t = splitLeaf(t, 'p1', 'row', 'p2');
    t = splitLeaf(t, 'p2', 'col', 'p3');
    expect(leafOrder(t)).toEqual(['p1', 'p2', 'p3']);
    assertWellFormed(t);
  });

  it('gives every split a distinct id, so ratios move independently', () => {
    let t: LayoutNode = leaf('p1');
    t = splitLeaf(t, 'p1', 'row', 'p2');
    t = splitLeaf(t, 'p2', 'col', 'p3');
    const ids: string[] = [];
    const walk = (n: LayoutNode) => {
      if (n.kind === 'split') { ids.push(n.id); walk(n.a); walk(n.b); }
    };
    walk(t);
    expect(new Set(ids).size).toBe(ids.length);
  });
});

describe('splitRoot', () => {
  it('wraps the whole tree so the new pane spans the edge', () => {
    const inner = splitLeaf(leaf('p1'), 'p1', 'row', 'p2');
    const t = splitRoot(inner, 'col', 'p3');
    expect(t.kind).toBe('split');
    if (t.kind === 'split') expect(t.a).toBe(inner);
    expect(leafOrder(t)).toEqual(['p1', 'p2', 'p3']);
    assertWellFormed(t);
  });
});

describe('removeLeaf', () => {
  it('collapses the parent split into the sibling', () => {
    const t = splitLeaf(leaf('p1'), 'p1', 'row', 'p2');
    const [next, survivor] = removeLeaf(t, 'p2');
    expect(next).toEqual(leaf('p1'));
    expect(survivor).toBe('p1');
    assertWellFormed(next);
  });

  it('never removes the root leaf — the last pane always stays', () => {
    const t = leaf('p1');
    const [next, survivor] = removeLeaf(t, 'p1');
    expect(next).toBe(t);
    expect(survivor).toBeNull();
  });

  it('is a no-op for a pane that is not in the tree', () => {
    const t = splitLeaf(leaf('p1'), 'p1', 'row', 'p2');
    const [next, survivor] = removeLeaf(t, 'ghost');
    expect(next).toBe(t);
    expect(survivor).toBeNull();
  });

  it('keeps every other pane when removing from a deep tree', () => {
    let t: LayoutNode = leaf('p1');
    t = splitLeaf(t, 'p1', 'row', 'p2');
    t = splitLeaf(t, 'p2', 'col', 'p3');
    t = splitLeaf(t, 'p3', 'row', 'p4');
    const [next, survivor] = removeLeaf(t, 'p3');
    expect(leafOrder(next)).toEqual(['p1', 'p2', 'p4']);
    expect(survivor).toBe('p4');
    assertWellFormed(next);
  });

  it('closing every pane in turn ends at a single leaf', () => {
    let t: LayoutNode = leaf('p1');
    t = splitLeaf(t, 'p1', 'row', 'p2');
    t = splitLeaf(t, 'p2', 'col', 'p3');
    for (const id of ['p2', 'p3']) {
      [t] = removeLeaf(t, id);
      assertWellFormed(t);
    }
    expect(t).toEqual(leaf('p1'));
  });

  it('reports a surviving pane whenever it removed one', () => {
    let t: LayoutNode = leaf('p1');
    t = splitLeaf(t, 'p1', 'row', 'p2');
    t = splitLeaf(t, 'p1', 'col', 'p3');
    for (const id of leafOrder(t)) {
      const [, survivor] = removeLeaf(t, id);
      expect(survivor, `removing ${id} reported no survivor`).not.toBeNull();
      expect(containsLeaf(t, survivor!)).toBe(true);
      expect(survivor).not.toBe(id);
    }
  });
});

describe('leafOrder', () => {
  it('reads left→right, top→bottom, which is what pane.next follows', () => {
    let t: LayoutNode = leaf('p1');
    t = splitLeaf(t, 'p1', 'row', 'p2');   // p1 | p2
    t = splitLeaf(t, 'p1', 'col', 'p3');   // (p1 / p3) | p2
    expect(leafOrder(t)).toEqual(['p1', 'p3', 'p2']);
  });
});

describe('firstLeaf', () => {
  it('descends the a-side', () => {
    let t: LayoutNode = leaf('p1');
    t = splitLeaf(t, 'p1', 'row', 'p2');
    t = splitLeaf(t, 'p1', 'col', 'p3');
    expect(firstLeaf(t)).toBe('p1');
  });
});

describe('setRatio', () => {
  it('clamps so neither side can collapse', () => {
    const t = splitLeaf(leaf('p1'), 'p1', 'row', 'p2');
    const id = t.kind === 'split' ? t.id : '';
    const low = setRatio(t, id, 0);
    const high = setRatio(t, id, 1);
    expect(low.kind === 'split' && low.ratio).toBe(0.15);
    expect(high.kind === 'split' && high.ratio).toBe(0.85);
  });

  it('touches only the named split', () => {
    let t: LayoutNode = leaf('p1');
    t = splitLeaf(t, 'p1', 'row', 'p2');
    t = splitLeaf(t, 'p2', 'col', 'p3');
    const outer = t.kind === 'split' ? t.id : '';
    const next = setRatio(t, outer, 0.7);
    expect(next.kind === 'split' && next.ratio).toBe(0.7);
    // The inner split kept its own ratio.
    const inner = next.kind === 'split' ? next.b : null;
    expect(inner && inner.kind === 'split' && inner.ratio).toBe(0.5);
  });

  it('is a no-op for an unknown split id', () => {
    const t = splitLeaf(leaf('p1'), 'p1', 'row', 'p2');
    expect(setRatio(t, 'split-nope', 0.7)).toBe(t);
  });
});
