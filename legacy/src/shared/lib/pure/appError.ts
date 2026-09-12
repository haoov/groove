// Reading a rejected `invoke`. A command rejects with `AppError`; anything else that
// throws here is a JS error or a string, and both must still render.

import type { AppError, ErrorKind } from '../../ipc/ipc';

function isAppError(e: unknown): e is AppError {
  return (
    typeof e === 'object' && e !== null &&
    typeof (e as AppError).message === 'string' &&
    typeof (e as AppError).kind === 'string'
  );
}

/** The human-readable message of any thrown value. */
export function errorText(e: unknown): string {
  if (isAppError(e)) return e.message;
  if (typeof e === 'string') return e;
  if (e instanceof Error) return e.message;
  if (typeof e === 'object' && e !== null) {
    const message = (e as { message?: unknown }).message;
    if (typeof message === 'string') return message;
  }
  return String(e);
}

/** The backend's error kind, or null when the failure did not come from a command. */
export function errorKind(e: unknown): ErrorKind | null {
  return isAppError(e) ? e.kind : null;
}

/** Whether a failure is the backend saying the thing is not there. */
export const isNotFound = (e: unknown) => errorKind(e) === 'not_found';
