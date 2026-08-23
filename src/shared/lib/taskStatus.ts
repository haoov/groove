// Task status names, normalized.
//
// Boards name their statuses freely ("Ready for sprint", "Fixed with required
// action"), so anything that colours or orders by status has to map the label to a
// meaning first. Shared by Home's queue and the task overview's status dot.

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

/** Compact label for the priority pill; '' when the task has no priority
 *  (which must stay distinct from "Low" — both rank 3). */
export function priorityLabel(priority: string | null): string {
  if (!priority) return '';
  return ['urg', 'high', 'med', 'low'][priorityRank(priority)] ?? '';
}
