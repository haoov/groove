import { Check, Minus, RotateCcw } from 'lucide-react';
import { FORGE_CLIS } from '../shared/lib/forge';
import type { Environment } from '../shared/ipc/ipc';

/** The same check the first-run screen does. Kept reachable afterwards: a tool
 *  uninstalled later breaks a feature with no other clue. The forge CLIs are not
 *  here — they are Git & forge's rows. */
export function MachinePanel({ env, reload }: { env: Environment | null; reload: () => void }) {
  const tools = (env?.tools ?? []).filter((t) => !FORGE_CLIS.includes(t.name));

  return (
    <section className="settings-section">
      <div className="settings-section-head">
        <div className="settings-section-title" style={{ marginBottom: 0 }}>Tools</div>
        <button className="settings-reset-btn" onClick={reload} title="Check again">
          <RotateCcw size={12} strokeWidth={1.75} /> Recheck
        </button>
      </div>
      {!env ? (
        <p className="settings-hint">Checking…</p>
      ) : (
        <>
          <ul className="firstrun-tools">
            {tools.map((t) => (
              <li key={t.name} className={t.path ? 'ok' : t.required ? 'missing' : 'optional'}>
                {t.path ? <Check size={12} strokeWidth={2.5} /> : <Minus size={12} strokeWidth={2} />}
                <code>{t.name}</code>
                <span className="firstrun-tool-purpose">{t.purpose}</span>
                {!t.path && <span className="firstrun-tool-tag">{t.required ? 'required' : 'optional'}</span>}
              </li>
            ))}
          </ul>
          <p className="settings-hint">
            Config: <code>{env.config_path}</code>
          </p>
        </>
      )}
    </section>
  );
}
