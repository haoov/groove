import { useEffect, type RefObject } from 'react';
import { ensureHost, fitAndSync } from './terminalHost';

/**
 * Show a PTY's terminal inside `containerRef`.
 *
 * The xterm instance lives in the module-level host registry, not in React, so
 * this only re-parents the host's element in and detaches it on unmount — which
 * is what lets a terminal survive tab switches, pane moves, and close/reopen
 * without losing its scrollback.
 *
 * An element can only be in one place at a time, so exactly one mounted consumer
 * may pass a non-null `ptySessionId` for a given session.
 */
export function useAttachedHost(
  ptySessionId: string | null,
  containerRef: RefObject<HTMLDivElement | null>,
  fontFamily?: string,
) {
  useEffect(() => {
    const container = containerRef.current;
    if (!ptySessionId || !container) return;
    const host = ensureHost(ptySessionId, fontFamily);
    container.appendChild(host.el);

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
      // Detach ONLY if we still hold it. Handing the terminal to another surface
      // means that surface appends the element before this cleanup runs — an
      // unconditional remove() would then rip it back out of its new home and
      // leave a blank terminal there. Never dispose: the registry owns it.
      if (host.el.parentElement === container) host.el.remove();
    };
  }, [ptySessionId, containerRef, fontFamily]);
}
