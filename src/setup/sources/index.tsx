import type { ComponentType } from 'react';
import type { Config, Environment, ProviderId } from '../../shared/ipc/ipc';
import { NotionSetupForm, NotionSettingsRow } from './NotionSetup';
import { GithubSetupForm, GithubSettingsRow } from './GithubSetup';

export interface SetupFormProps {
  /** The provider's setup payload as `set_task_source`/`write_initial_config`
   *  take it; null while the form is incomplete (which blocks saving). */
  onChange: (payload: unknown | null) => void;
  /** The gh CLI needs a login or a wider scope — reopen the auth flow. */
  onNeedsScope: () => void;
}

export interface SettingsRowProps {
  config: Config | null;
  env: Environment | null;
  busy: boolean;
  /** invoke('set_task_source') for this provider, wrapped by the settings page. */
  setSource: (enabled: boolean, options: unknown) => Promise<void>;
  onNeedsScope: () => void;
}

interface SourceModule {
  /** Display name — the first-run section heading. */
  label: string;
  SetupForm: ComponentType<SetupFormProps>;
  SettingsRow: ComponentType<SettingsRowProps>;
}

/** Every task source the app can connect. Keyed by ProviderId so a provider
 *  added on the Rust side fails the build here until its components exist —
 *  same pattern as PROVIDERS in shared/lib/taskProvider.ts. */
export const SOURCES: Record<ProviderId, SourceModule> = {
  notion: { label: 'Notion', SetupForm: NotionSetupForm, SettingsRow: NotionSettingsRow },
  github: { label: 'GitHub Projects', SetupForm: GithubSetupForm, SettingsRow: GithubSettingsRow },
};

export const SOURCE_IDS = Object.keys(SOURCES) as ProviderId[];
