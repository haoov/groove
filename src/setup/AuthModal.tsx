import { useEffect, useRef, useState } from 'react';
import { Modal } from '../shared/ui/Modal';
import { invoke } from '../shared/ipc/invoke';
import { listen } from '@tauri-apps/api/event';
import { Loader2, X } from 'lucide-react';
import { useStore } from '../shared/store';
import { EVENT } from '../shared/ipc/events';
import type { PtyOutputEvent } from '../shared/ipc/ipc';
import { focusHost } from '../shared/lib/hosts/terminalHost';
import { useAttachedHost } from '../shared/lib/hooks/useAttachedHost';
import { bytesToB64 } from '../shared/lib/hosts/ptyRegistry';

/** Typed at the prompt without a newline. */
const COMMAND = {
  glab: { login: 'glab auth login', scope: 'glab auth login' },
  gh: { login: 'gh auth login', scope: 'gh auth refresh -s project' },
} as const;

export type AuthMode = 'login' | 'scope';

/** PTY quiet time that signals the prompt is ready. */
const SETTLE_MS = 400;

/**
 * A shell for signing the forge CLIs in. The command is typed at the prompt without
 * its newline. The PTY is owned here and dies with the modal.
 */
export function AuthModal({
  tool, mode = 'login', onDone,
}: {
  tool: 'glab' | 'gh';
  /** `scope` widens an existing login instead of starting one. */
  mode?: AuthMode;
  onDone: () => void;
}) {
  const setLastError = useStore((s) => s.setLastError);
  const [pty, setPty] = useState<string | null>(null);
  const termRef = useRef<HTMLDivElement>(null);
  useAttachedHost(pty, termRef);

  // One start per open; a second orphans the first PTY.
  const started = useRef(false);
  useEffect(() => {
    if (started.current) return;
    started.current = true;
    invoke<string>('start_auth_session')
      .then((id) => { setPty(id); focusHost(id); })
      .catch((e) => { setLastError(e); onDone(); });
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tool]);

  // Type the command once the PTY falls quiet: text written mid-burst is lost.
  const typed = useRef(false);
  useEffect(() => {
    if (!pty) return;
    let quiet: number | undefined;
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    const arm = () => {
      window.clearTimeout(quiet);
      quiet = window.setTimeout(() => {
        if (typed.current) return;
        typed.current = true;
        const dataB64 = bytesToB64(new TextEncoder().encode(COMMAND[tool][mode]));
        invoke('write_pty', { sessionId: pty, dataB64 }).catch(() => { /* the user can type it */ });
        focusHost(pty);
      }, SETTLE_MS);
    };
    listen<PtyOutputEvent>(EVENT.PTY_OUTPUT, ({ payload }) => {
      if (payload.session_id === pty) arm();
    }).then((un) => {
      if (cancelled) un();
      else unlisten = un;
    });
    arm(); // a silent shell still gets the command
    return () => { cancelled = true; window.clearTimeout(quiet); unlisten?.(); };
  }, [pty, tool, mode]);

  return (
    <Modal
      className="auth-modal"
      title={`Sign in to ${tool === 'glab' ? 'GitLab' : 'GitHub'}`}
      subtitle={
        <>
          <code>{COMMAND[tool][mode]}</code> is ready below — add any flags you need,
          then press Enter.
        </>
      }
      onClose={onDone}
    >
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
    </Modal>
  );
}
