import type { ComponentType } from 'react';
import type { Config, Environment, ProviderId } from '../../shared/ipc/ipc';
import { NotionSetupForm, NotionSettingsRow } from './NotionSetup';
import { GithubSetupForm, GithubSettingsRow } from './GithubSetup';

export interface SetupFormProps {
  /** The provider's setup payload; null while the form is incomplete. */
  onChange: (payload: unknown | null) => void;
  /** The gh CLI needs a login or a wider scope. */
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
  /** Display name. */
  label: string;
  SetupForm: ComponentType<SetupFormProps>;
  SettingsRow: ComponentType<SettingsRowProps>;
}

/** Every task source, keyed by ProviderId: a new provider fails the build until its components exist. */
export const SOURCES: Record<ProviderId, SourceModule> = {
  notion: { label: 'Notion', SetupForm: NotionSetupForm, SettingsRow: NotionSettingsRow },
  github: { label: 'GitHub Projects', SetupForm: GithubSetupForm, SettingsRow: GithubSettingsRow },
};

export const SOURCE_IDS = Object.keys(SOURCES) as ProviderId[];
