/** Collapse a forge pipeline status into a tone for the CI badge. */
export function ciTone(status: string): 'good' | 'bad' | 'run' | 'muted' {
  const s = status.toLowerCase();
  if (['success', 'passed', 'completed'].includes(s)) return 'good';
  if (['failed', 'error', 'canceled', 'cancelled'].includes(s)) return 'bad';
  if (['running', 'pending', 'created', 'in_progress', 'queued'].includes(s)) return 'run';
  return 'muted';
}
