// Task status names, normalized: a board's free-form label maps to one known key.

/** Order tasks appear in the queue. Lower sorts first. */
export const STATUS_RANK: Record<string, number> = {
  in_progress: 0,
  ready: 1,
  blocked: 2,
  in_review: 3,
};

export function statusKey(status: string): string {
  const s = status.toLowerCase().replace(/\s+/g, '_');
  if (s.includes('progress')) return 'in_progress';
  if (s.includes('blocked')) return 'blocked';
  if (s.includes('review')) return 'in_review';
  if (s.includes('done') || s.includes('complete')) return 'done';
  return 'ready';
}

/** Priority text → rank 0..3 (urgent..low). Unset ranks 3, like Low. */
export function priorityRank(priority: string | null): number {
  const s = (priority ?? '').toLowerCase();
  if (s.includes('p0') || s.includes('urgent') || s.includes('critical') || s.includes('blocker')) return 0;
  if (s.includes('p1') || s.includes('high')) return 1;
  if (s.includes('p2') || s.includes('medium') || s.includes('normal')) return 2;
  return 3;
}

/** Compact label for the priority pill; '' for no priority, which stays distinct from Low. */
export function priorityLabel(priority: string | null): string {
  if (!priority) return '';
  return ['urg', 'high', 'med', 'low'][priorityRank(priority)] ?? '';
}
