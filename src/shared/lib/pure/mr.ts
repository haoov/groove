/** Group a raw CI status string for styling (forge-ci-* classes). */
export function ciGroup(status: string): 'ok' | 'fail' | 'run' | 'idle' {
  if (status === 'success') return 'ok';
  if (status === 'failed') return 'fail';
  if (['running', 'pending', 'preparing', 'created', 'waiting_for_resource', 'scheduled'].includes(status)) return 'run';
  return 'idle';
}
