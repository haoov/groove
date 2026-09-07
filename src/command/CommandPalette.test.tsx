// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent, waitFor } from '@testing-library/react';
import { useStore } from '../shared/store';
import { resetStore, seedSession } from '../shared/store/testing';
import { THEMES } from '../shared/ipc/ipc';
import { CommandPalette } from './CommandPalette';

vi.mock('../shared/ipc/invoke', () => ({ invoke: vi.fn() }));
import { invoke } from '../shared/ipc/invoke';

const invoked = vi.mocked(invoke);

// jsdom ships no scrollIntoView.
Element.prototype.scrollIntoView = () => {};

const input = () => screen.getByPlaceholderText('Type a command…');
const type = (query: string) => fireEvent.change(input(), { target: { value: query } });
const press = (key: string) => fireEvent.keyDown(input(), { key });
const rows = () => screen.getAllByRole('button').map((b) => b.textContent ?? '');

beforeEach(() => {
  resetStore();
  seedSession();
  invoked.mockReset();
  invoked.mockResolvedValue(undefined as never);
  useStore.setState({ commandPaletteOpen: true, view: 'workspace' });
});

afterEach(cleanup);

describe('the command palette', () => {
  it('shows nothing while closed', () => {
    useStore.setState({ commandPaletteOpen: false });
    const { container } = render(<CommandPalette />);
    expect(container.firstChild).toBeNull();
  });

  it('lists the keybound commands of the registry', () => {
    render(<CommandPalette />);
    expect(screen.getByText('Home')).toBeTruthy();
    expect(screen.getByText('Open settings…')).toBeTruthy();
    expect(screen.getByText('Source control')).toBeTruthy();
  });

  it('never offers itself', () => {
    render(<CommandPalette />);
    expect(screen.queryByText('Command palette')).toBeNull();
  });

  it('shows a command shortcut beside its row', () => {
    render(<CommandPalette />);
    expect(screen.getByText('Home').closest('button')?.textContent).toContain('Alt+T');
  });

  it('narrows to one command on the initials of its words', () => {
    render(<CommandPalette />);
    type('gcm');

    expect(screen.getByText('1 command')).toBeTruthy();
    expect(rows()[0]).toContain('Git: Create merge request… — mayo');
  });

  it('drops the commands the query does not match', () => {
    render(<CommandPalette />);
    type('gcm');

    expect(screen.queryByText('Home')).toBeNull();
    expect(rows().some((r) => r.includes('Git: Push'))).toBe(false);
  });

  it('says so when nothing matches', () => {
    render(<CommandPalette />);
    type('zzqq');
    expect(screen.getByText('No commands match')).toBeTruthy();
  });

  it('runs the first match on Enter', async () => {
    render(<CommandPalette />);
    type('theme:');
    press('Enter');

    await waitFor(() => expect(invoked).toHaveBeenCalledWith('set_theme', { theme: THEMES[0].id }));
  });

  it('runs the next match after ArrowDown', async () => {
    render(<CommandPalette />);
    type('theme:');
    press('ArrowDown');
    press('Enter');

    await waitFor(() => expect(invoked).toHaveBeenCalledWith('set_theme', { theme: THEMES[1].id }));
  });

  it('walks back to the first match on ArrowUp', async () => {
    render(<CommandPalette />);
    type('theme:');
    press('ArrowDown');
    press('ArrowDown');
    press('ArrowUp');
    press('Enter');

    await waitFor(() => expect(invoked).toHaveBeenCalledWith('set_theme', { theme: THEMES[1].id }));
  });

  it('holds the selection at the last match', async () => {
    render(<CommandPalette />);
    type('theme:');
    for (let i = 0; i < THEMES.length + 3; i++) press('ArrowDown');
    press('Enter');

    await waitFor(() =>
      expect(invoked).toHaveBeenCalledWith('set_theme', { theme: THEMES[THEMES.length - 1].id }),
    );
  });

  it('restarts the selection at the top when the query changes', async () => {
    render(<CommandPalette />);
    type('theme:');
    press('ArrowDown');
    press('ArrowDown');
    press('ArrowDown');
    type('theme: catppuccin');
    press('Enter');

    await waitFor(() => expect(invoked).toHaveBeenCalledWith('set_theme', { theme: THEMES[0].id }));
  });

  it('closes itself once a command runs', async () => {
    render(<CommandPalette />);
    type('theme:');
    press('Enter');

    await waitFor(() => expect(useStore.getState().commandPaletteOpen).toBe(false));
  });

  it('runs a command on a click', () => {
    render(<CommandPalette />);
    fireEvent.click(screen.getByText('Home'));

    expect(useStore.getState().view).toBe('home');
    expect(useStore.getState().commandPaletteOpen).toBe(false);
  });

  it('closes on Escape without running anything', () => {
    render(<CommandPalette />);
    fireEvent.keyDown(window, { key: 'Escape' });

    expect(useStore.getState().commandPaletteOpen).toBe(false);
    expect(invoked).not.toHaveBeenCalled();
  });

  it('hides the session commands when no session is open', () => {
    resetStore();
    useStore.setState({ commandPaletteOpen: true, view: 'home' });
    render(<CommandPalette />);

    expect(screen.getByText('Home')).toBeTruthy();
    expect(screen.queryByText('Source control')).toBeNull();
    expect(rows().some((r) => r.includes('Git: Push'))).toBe(false);
  });
});
