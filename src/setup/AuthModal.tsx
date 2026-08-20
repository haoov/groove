import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Loader2, X } from 'lucide-react';
import { useStore } from '../shared/store';
import { EVENT } from '../shared/ipc/events';
import type { PtyOutputEvent } from '../shared/ipc/ipc';
import { focusHost } from '../terminal/terminalHost';
import { useAttachedHost } from '../terminal/useAttachedHost';

/** Typed at the prompt without a newline: the command is the same for everyone but
 *  its flags are not — a self-managed GitLab needs `--hostname`, and which login
 *  method works there is the user's call. */
const LOGIN_COMMAND = { glab: 'glab auth login', gh: 'gh auth login' } as const;

/** Quiet from the PTY before the command is typed. A shell's startup arrives in
 *  bursts, and quiet is the only signal it gives that the prompt is ready. */
const SETTLE_MS = 400;

/**
 * A shell for signing the forge CLIs in.
 *
 * Both flows are interactive by nature — a device code to copy, a browser to open, a
 * token to paste — so there is nothing to wrap in a form. Showing the CLI itself is
 * both less work and more honest: the user sees exactly what it asked and what it
 * answered, and the same commands work outside the app.
 *
 * A shell rather than the login command itself, because the command varies: a
 * self-managed GitLab needs its host, and `--hostname` is only the first flag someone
 * will need. So the command is typed at the prompt without its newline — ready to run,
 * still open to editing.
 *
 * The PTY is not bound to a session, so this owns it: it dies with the login.
 */
export function AuthModal({ tool, onDone }: { tool: 'glab' | 'gh'; onDone: () => void }) {
  const setLastError = useStore((s) => s.setLastError);
  const [pty, setPty] = useState<string | null>(null);
  const termRef = useRef<HTMLDivElement>(null);
  useAttachedHost(pty, termRef);

  // One login per open — a second start would leave the first PTY orphaned.
  const started = useRef(false);
  useEffect(() => {
    if (started.current) return;
    started.current = true;
    invoke<string>('start_auth_session')
      .then((id) => { setPty(id); focusHost(id); })
      .catch((e) => { setLastError(String(e)); onDone(); });
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tool]);

  // Type the command at the prompt, leaving the newline to the user so they can add
  // flags first. Waiting for the PTY to fall quiet rather than firing on the first
  // chunk: startup output comes in bursts, and text written into the middle of it is
  // simply lost.
  const typed = useRef(false);
  useEffect(() => {
    if (!pty) return;
    let quiet: number | undefined;
    let unlisten: (() => void) | undefined;
    const arm = () => {
      window.clearTimeout(quiet);
      quiet = window.setTimeout(() => {
        if (typed.current) return;
        typed.current = true;
        const data = Array.from(new TextEncoder().encode(LOGIN_COMMAND[tool]));
        invoke('write_pty', { sessionId: pty, data }).catch(() => { /* the user can type it */ });
        focusHost(pty);
      }, SETTLE_MS);
    };
    listen<PtyOutputEvent>(EVENT.PTY_OUTPUT, ({ payload }) => {
      if (payload.session_id === pty) arm();
    }).then((un) => { unlisten = un; });
    arm(); // a shell that prints nothing at all still gets the command
    return () => { window.clearTimeout(quiet); unlisten?.(); };
  }, [pty, tool]);

  // Closing is deliberately the user's call: a shell has no "done", and the login
  // may take a browser round trip. `onDone` re-runs the environment check, so a
  // half-finished login simply shows as still not authenticated.

  return (
    <div className="wizard-overlay" onClick={onDone}>
      <div className="wizard-modal auth-modal" onClick={(e) => e.stopPropagation()}>
        <div className="wizard-header">
          <div className="wizard-title">Sign in to {tool === 'glab' ? 'GitLab' : 'GitHub'}</div>
          <div className="wizard-subtitle">
            <code>{LOGIN_COMMAND[tool]}</code> is ready below — add any flags you need,
            then press Enter.
          </div>
          <button className="wizard-close" onClick={onDone}>×</button>
        </div>
        <div className="auth-term console-term" ref={termRef}>
          {!pty && (
            <span className="console-hint">
              <Loader2 size={12} className="spin" /> Starting a shell…
            </span>
          )}
        </div>
        <div className="wizard-footer">
          <span className="firstrun-hint" style={{ margin: 0 }}>
            Close this when the CLI says you are logged in; the check re-runs.
          </span>
          <span className="composer-spacer" />
          <button className="btn-primary" onClick={onDone}>
            <X size={11} strokeWidth={2} style={{ marginRight: 5 }} />
            Done
          </button>
        </div>
      </div>
    </div>
  );
}
