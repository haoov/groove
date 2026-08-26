import { useCallback, useEffect, useState } from 'react';
import { X } from 'lucide-react';
import { invoke } from '../shared/ipc/invoke';
import { useStore } from '../shared/store';
import { AuthModal } from '../setup/AuthModal';
import { SETTINGS_GROUPS, DEFAULT_SETTINGS_GROUP, type SettingsGroupId } from './groups';
import { AppearancePanel } from './AppearancePanel';
import { EditorPanel } from './EditorPanel';
import { ActionsPanel } from './ActionsPanel';
import { ShortcutsPanel } from './ShortcutsPanel';
import { SourcesPanel } from './SourcesPanel';
import { GitForgePanel } from './GitForgePanel';
import { MachinePanel } from './MachinePanel';
import type { Config, Environment } from '../shared/ipc/ipc';

/** Groups whose panel reads `check_environment`. */
const ENV_GROUPS: SettingsGroupId[] = ['sources', 'git', 'machine'];

/** Preferences as a view: the groups on the left, one group's settings on the right. */
export function SettingsView() {
  const config = useStore((s) => s.config);
  const setConfig = useStore((s) => s.setConfig);
  const closeSettings = useStore((s) => s.closeSettings);

  const [group, setGroup] = useState<SettingsGroupId>(DEFAULT_SETTINGS_GROUP);
  const [authing, setAuthing] = useState<{ tool: 'glab' | 'gh'; mode: 'login' | 'scope' } | null>(null);
  const [env, setEnv] = useState<Environment | null>(null);

  const loadEnv = useCallback(() => {
    invoke<Environment>('check_environment').then(setEnv).catch(() => setEnv(null));
  }, []);

  // Loaded on demand — each check shells out to `gh` / `glab auth status`.
  const needsEnv = ENV_GROUPS.includes(group);
  useEffect(() => {
    if (needsEnv && !env) loadEnv();
  }, [needsEnv, env, loadEnv]);

  const reload = useCallback(() => {
    invoke<Config | null>('get_config').then((c) => { if (c) setConfig(c); }).catch(() => {});
    loadEnv();
  }, [setConfig, loadEnv]);

  const active = SETTINGS_GROUPS.find((g) => g.id === group) ?? SETTINGS_GROUPS[0];

  return (
    <div className="settings-view">
      {authing && (
        <AuthModal
          tool={authing.tool}
          mode={authing.mode}
          onDone={() => { setAuthing(null); reload(); }}
        />
      )}

      <nav className="settings-nav" aria-label="Settings groups">
        <div className="settings-nav-title">Settings</div>
        {SETTINGS_GROUPS.map((g) => {
          const Icon = g.icon;
          return (
            <button
              key={g.id}
              className={`settings-nav-item ${g.id === group ? 'selected' : ''}`}
              onClick={() => setGroup(g.id)}
              aria-current={g.id === group}
            >
              <Icon size={15} strokeWidth={1.75} />
              <span className="settings-nav-text">
                <span className="settings-nav-label">{g.label}</span>
                <span className="settings-nav-blurb">{g.blurb}</span>
              </span>
            </button>
          );
        })}
      </nav>

      <div className="settings-panel">
        <header className="settings-panel-head">
          <h2 className="settings-panel-title">{active.label}</h2>
          <button
            className="settings-close"
            onClick={closeSettings}
            aria-label="Close settings"
            title="Close settings"
          >
            <X size={16} strokeWidth={1.75} />
          </button>
        </header>

        <div className="settings-panel-body">
          {group === 'appearance' && <AppearancePanel />}
          {group === 'editor' && <EditorPanel />}
          {group === 'actions' && <ActionsPanel />}
          {group === 'shortcuts' && <ShortcutsPanel />}
          {group === 'sources' && (
            <SourcesPanel
              config={config}
              env={env}
              reload={reload}
              onNeedsScope={() => setAuthing({ tool: 'gh', mode: 'scope' })}
            />
          )}
          {group === 'git' && (
            <GitForgePanel
              config={config}
              env={env}
              onSignIn={(tool, mode) => setAuthing({ tool, mode })}
            />
          )}
          {group === 'machine' && <MachinePanel env={env} reload={reload} />}
        </div>
      </div>
    </div>
  );
}
