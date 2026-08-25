import { useState } from 'react';
import { invoke } from '../../shared/ipc/invoke';
import { SOURCES, SOURCE_IDS } from './index';
import type { Config, Environment, ProviderId } from '../../shared/ipc/ipc';

/** Which sources are on, what they point at, and how to fix a source that has
 *  stopped working. Not a second setup screen — the same shape as "This machine". */
export function TaskSources({
  config, env, onChanged, onNeedsScope,
}: {
  config: Config | null;
  env: Environment | null;
  onChanged: () => void;
  onNeedsScope: () => void;
}) {
  const [busy, setBusy] = useState<ProviderId | null>(null);
  const [error, setError] = useState<string | null>(null);

  const setSource = (provider: ProviderId) => async (enabled: boolean, options: unknown) => {
    setBusy(provider);
    setError(null);
    try {
      await invoke('set_task_source', { provider, enabled, options });
      onChanged();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <>
      <ul className="firstrun-tools">
        {SOURCE_IDS.map((id) => {
          const Row = SOURCES[id].SettingsRow;
          return (
            <Row
              key={id}
              config={config}
              env={env}
              busy={busy === id}
              setSource={setSource(id)}
              onNeedsScope={onNeedsScope}
            />
          );
        })}
      </ul>

      {error && <p className="settings-hint settings-error">{error}</p>}
      <p className="settings-hint">
        Tasks from every connected source share one queue. Removing the last one is refused —
        the app has nothing to show without it.
      </p>
    </>
  );
}
