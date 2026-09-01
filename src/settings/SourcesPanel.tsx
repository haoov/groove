import { TaskSources } from '../setup/sources/TaskSources';
import type { Config, Environment } from '../shared/ipc/ipc';

/** Status and repair, not a second setup screen. The first-run screen is
 *  unreachable once configured, so this is the only route to adding a source to a
 *  machine that already has one. */
export function SourcesPanel({
  config, env, reload, onNeedsScope,
}: {
  config: Config | null;
  env: Environment | null;
  reload: () => void;
  onNeedsScope: () => void;
}) {
  return (
    <section className="settings-section">
      <div className="settings-section-title">Connected sources</div>
      <TaskSources
        config={config}
        env={env}
        onChanged={reload}
        onNeedsScope={onNeedsScope}
      />
    </section>
  );
}
