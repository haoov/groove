import { Palette, SquareCode, Keyboard, Inbox, GitBranch, MonitorCog, type LucideIcon } from 'lucide-react';

export type SettingsGroupId =
  | 'appearance'
  | 'editor'
  | 'shortcuts'
  | 'sources'
  | 'git'
  | 'machine';

export interface SettingsGroup {
  id: SettingsGroupId;
  label: string;
  icon: LucideIcon;
  /** One-line sidebar subtitle. */
  blurb: string;
}

/** The sidebar, in display order. */
export const SETTINGS_GROUPS: SettingsGroup[] = [
  { id: 'appearance', label: 'Appearance', icon: Palette, blurb: 'Theme, font and size' },
  { id: 'editor', label: 'Editor', icon: SquareCode, blurb: 'Editing behaviour' },
  { id: 'shortcuts', label: 'Shortcuts', icon: Keyboard, blurb: 'Every global keybinding' },
  { id: 'sources', label: 'Task sources', icon: Inbox, blurb: 'Notion and GitHub' },
  { id: 'git', label: 'Git & forge', icon: GitBranch, blurb: 'Worktree root and sign-in' },
  { id: 'machine', label: 'This machine', icon: MonitorCog, blurb: 'Tools and config file' },
];

export const DEFAULT_SETTINGS_GROUP: SettingsGroupId = 'appearance';
