import { useEffect, type RefObject } from 'react';
import { ensureHost, fitAndSync, forgetSyncedSize } from './terminalHost';

/**
 * Shows a PTY's terminal inside `containerRef` by re-parenting the host element.
 * Exactly one mounted consumer may pass a non-null `ptySessionId` for a session.
 */
export function useAttachedHost(
  ptySessionId: string | null,
  containerRef: RefObject<HTMLDivElement | null>,
  /** The agent's terminal, with its own font. */
  agent = false,
) {
  useEffect(() => {
    const container = containerRef.current;
    if (!ptySessionId || !container) return;
    const host = ensureHost(ptySessionId, agent);
    container.appendChild(host.el);
    // Another window may have resized the PTY; the record must not skip the first fit.
    forgetSyncedSize(ptySessionId);

    let raf1 = 0;
    let raf2 = 0;
    // Two frames: one for the container to lay out, one for xterm to measure it.
    const refit = () => {
      raf1 = requestAnimationFrame(() => {
        raf2 = requestAnimationFrame(() => fitAndSync(ptySessionId));
      });
    };
    refit();
    const ro = new ResizeObserver(refit);
    ro.observe(container);

    return () => {
      cancelAnimationFrame(raf1);
      cancelAnimationFrame(raf2);
      ro.disconnect();
      // Detach only while this container still holds it: another surface may have appended it already.
      // Never dispose: the registry owns it.
      if (host.el.parentElement === container) host.el.remove();
    };
  }, [ptySessionId, containerRef, agent]);
}
