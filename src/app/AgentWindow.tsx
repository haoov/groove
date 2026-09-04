import { useCallback, useEffect, useRef, useState } from 'react';
import { emitTo, listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { PanelRight } from 'lucide-react';
import { useStore } from '../shared/store';
import { EVENT } from '../shared/ipc/events';
import { deliverPtyOutput } from '../shared/lib/ptyRegistry';
import { disposeHost, focusHost } from '../shared/lib/terminalHost';
import { useAttachedHost } from '../shared/lib/useAttachedHost';
import { applyTheme, applyFontSize, applyFontFamily } from '../shared/lib/theme';
import { isMac } from '../shared/lib/platform';
import {
  BRIDGE,
  type AgentWindowCommand,
  type AgentWindowState,
  type CommandDone,
  type CommandEnvelope,
} from '../shared/lib/agentWindow';
import { AgentPanel, AGENT_FONT } from '../agent/AgentPanel';
import { SOURCE_IDS } from '../setup/sources';
import { WindowControls } from './chrome/WindowControls';
import { ResizeHandles } from './chrome/ResizeHandles';
import type { PtyExitEvent, PtyOutputEvent } from '../shared/ipc/ipc';

const MAIN_WINDOW = 'main';
/** A command the main window never answers is an error, not a stuck spinner. */
const COMMAND_TIMEOUT_MS = 30_000;
/** Quiet time after the last move or resize before the bounds are saved. */
const BOUNDS_SETTLE_MS = 400;

/**
 * The detached agent window: the agent panel alone, over the focused session's
 * agent. It draws the state the main window mirrors into it and owns only its
 * xterm and its geometry; every action goes back to the main window to run.
 */
export default function AgentWindow() {
  const [state, setState] = useState<AgentWindowState | null>(null);
  const setConfig = useStore((s) => s.setConfig);
  const termRef = useRef<HTMLDivElement>(null);
  const ptyId = state?.ptyId ?? null;
  const ptyIdRef = useRef<string | null>(null);
  ptyIdRef.current = ptyId;

  // Pending commands, resolved by the main window's DONE.
  const pending = useRef(new Map<number, { resolve: () => void; reject: (e: Error) => void }>());
  const nonce = useRef(0);
  const send = useCallback((command: AgentWindowCommand) => {
    return new Promise<void>((resolve, reject) => {
      const id = ++nonce.current;
      const timer = window.setTimeout(() => {
        pending.current.delete(id);
        reject(new Error('no answer from the main window'));
      }, COMMAND_TIMEOUT_MS);
      pending.current.set(id, {
        resolve: () => { window.clearTimeout(timer); resolve(); },
        reject: (e) => { window.clearTimeout(timer); reject(e); },
      });
      const envelope: CommandEnvelope = { nonce: id, command };
      emitTo(MAIN_WINDOW, BRIDGE.COMMAND, envelope).catch((e) => {
        pending.current.delete(id);
        window.clearTimeout(timer);
        reject(e instanceof Error ? e : new Error(String(e)));
      });
    });
  }, []);

  useEffect(() => {
    let cancelled = false;
    const unlisten: Array<() => void> = [];
    const track = (fn: () => void) => {
      if (cancelled) fn();
      else unlisten.push(fn);
    };
    (async () => {
      track(await listen<AgentWindowState>(BRIDGE.STATE, ({ payload }) => setState(payload)));
      track(await listen<CommandDone>(BRIDGE.DONE, ({ payload }) => {
        const p = pending.current.get(payload.nonce);
        if (!p) return;
        pending.current.delete(payload.nonce);
        if (payload.error) p.reject(new Error(payload.error));
        else p.resolve();
      }));
      // Only the agent's own stream: every PTY broadcasts here, and buffering the
      // terminals' output for a handler that never mounts is waste.
      track(await listen<PtyOutputEvent>(EVENT.PTY_OUTPUT, ({ payload }) => {
        if (payload.session_id === ptyIdRef.current) deliverPtyOutput(payload.session_id, payload.b64);
      }));
      track(await listen<PtyExitEvent>(EVENT.PTY_EXIT, ({ payload }) => disposeHost(payload.session_id)));
      await emitTo(MAIN_WINDOW, BRIDGE.READY, null);
    })().catch(console.error);
    return () => {
      cancelled = true;
      unlisten.forEach((u) => u());
    };
  }, []);

  // The mirrored config drives the theme, the font ramp, and — through the store
  // copy — xterm's font size and re-skin.
  const config = state?.config ?? null;
  useEffect(() => {
    if (!config) return;
    applyTheme(config.ui.theme);
    applyFontSize(config.ui.font_size);
    applyFontFamily(config.ui.font_family);
    setConfig(config);
  }, [config, setConfig]);

  useAttachedHost(ptyId, termRef, AGENT_FONT);
  useEffect(() => {
    if (ptyId) focusHost(ptyId);
  }, [ptyId]);

  // Geometry goes to the main window, which owns the saved layout.
  useEffect(() => {
    const w = getCurrentWindow();
    let timer: number | undefined;
    const report = () => {
      window.clearTimeout(timer);
      timer = window.setTimeout(async () => {
        try {
          const scale = await w.scaleFactor();
          const pos = (await w.outerPosition()).toLogical(scale);
          const size = (await w.innerSize()).toLogical(scale);
          await send({ type: 'bounds', bounds: { x: pos.x, y: pos.y, width: size.width, height: size.height } });
        } catch {
          /* a window mid-close has no geometry */
        }
      }, BOUNDS_SETTLE_MS);
    };
    const moved = w.onMoved(report);
    const resized = w.onResized(report);
    return () => {
      window.clearTimeout(timer);
      moved.then((u) => u()).catch(() => {});
      resized.then((u) => u()).catch(() => {});
    };
  }, [send]);

  const sources = SOURCE_IDS.filter((id) => !!config?.[id]);
  const headActions = (
    <>
      <button
        className="dock-close"
        onClick={() => void send({ type: 'dock' })}
        title="Dock the agent back into the main window"
      >
        <PanelRight size={12} strokeWidth={2} />
      </button>
      {/* macOS has its own traffic lights. */}
      {!isMac() && <WindowControls />}
    </>
  );

  return (
    <div className="agent-window">
      {state?.taskId ? (
        <AgentPanel
          taskId={state.taskId}
          ptyId={state.ptyId}
          kind={state.kind}
          activity={state.activity}
          skills={state.skills}
          skillsStale={state.skillsStale}
          sources={sources}
          autoApprove={state.autoApprove}
          termRef={termRef}
          onStart={() => send({ type: 'start' })}
          onRunSkill={(skillId, args) => send({ type: 'skill', skillId, args })}
          onReload={() => send({ type: 'reload' })}
          onSetAutoApprove={(value) => void send({ type: 'autoApprove', value })}
          headActions={headActions}
          headIsTitleBar
        />
      ) : (
        <>
          <div className="agent-pane-head" data-tauri-drag-region>
            <span className="pill-dot" />
            <span className="console-status" data-tauri-drag-region>no session focused</span>
            {headActions}
          </div>
          <div className="console-term">
            <span className="console-hint">Focus a session in the main window</span>
          </div>
        </>
      )}
      {!isMac() && <ResizeHandles />}
    </div>
  );
}
