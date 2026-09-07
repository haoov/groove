// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent, waitFor } from '@testing-library/react';
import { useStore } from '../shared/store';
import { resetStore, seedSession } from '../shared/store/testing';
import { OP } from '../shared/ipc/ops';
import type { ConfirmationDto } from '../shared/ipc/ipc';
import { ConfirmModal } from './ConfirmModal';

vi.mock('../shared/ipc/invoke', () => ({ invoke: vi.fn() }));
import { invoke } from '../shared/ipc/invoke';

const invoked = vi.mocked(invoke);

const commitRequest: ConfirmationDto = {
  id: 'c1',
  session_id: null,
  op_type: OP.GIT_COMMIT,
  payload: { repo: 'mayo', branch: 'fix/parser', message: 'fix: drop the stale ref' },
  origin: 'mcp',
};

const closeRequest: ConfirmationDto = {
  id: 'c2',
  session_id: null,
  op_type: OP.MR_CLOSE,
  payload: { repo: 'mayo', branch: 'fix/parser' },
  origin: 'ui',
};

const queue = (...requests: ConfirmationDto[]) =>
  requests.forEach((r) => useStore.getState().addConfirmation(r));

const resolveCalls = () => invoked.mock.calls.filter(([cmd]) => cmd === 'resolve_confirmation');

const lastResolve = () => {
  const calls = resolveCalls();
  return calls[calls.length - 1]?.[1] as { id: string; approved: boolean } | undefined;
};

beforeEach(() => {
  resetStore();
  seedSession();
  invoked.mockReset();
  invoked.mockResolvedValue(undefined as never);
});

afterEach(cleanup);

describe('the confirmation modal', () => {
  it('shows nothing while no request is pending', () => {
    const { container } = render(<ConfirmModal />);
    expect(container.firstChild).toBeNull();
  });

  it('renders the op label and its payload view', () => {
    queue(commitRequest);
    render(<ConfirmModal />);

    expect(screen.getByText('Git commit')).toBeTruthy();
    expect(screen.getByText('mayo')).toBeTruthy();
    expect(screen.getByText('fix/parser')).toBeTruthy();
    expect(screen.getByRole('textbox')).toHaveProperty('value', 'fix: drop the stale ref');
  });

  it('names the agent as the origin of an agent request', () => {
    queue(commitRequest);
    render(<ConfirmModal />);
    expect(screen.getByText('Agent')).toBeTruthy();
  });

  it('approves on Enter and drops the request', async () => {
    queue(commitRequest);
    render(<ConfirmModal />);

    fireEvent.keyDown(window, { key: 'Enter' });
    await waitFor(() => expect(lastResolve()).toMatchObject({ id: 'c1', approved: true }));
    await waitFor(() => expect(useStore.getState().pendingConfirmations).toHaveLength(0));
    expect(screen.queryByText('Git commit')).toBeNull();
  });

  it('approves on Ctrl+Enter from inside an edited field', async () => {
    queue(commitRequest);
    render(<ConfirmModal />);
    screen.getByRole('textbox').focus();

    fireEvent.keyDown(window, { key: 'Enter', ctrlKey: true });
    await waitFor(() => expect(lastResolve()).toMatchObject({ id: 'c1', approved: true }));
  });

  it('never approves on a plain Enter typed into a field', async () => {
    queue(commitRequest);
    render(<ConfirmModal />);
    screen.getByRole('textbox').focus();

    fireEvent.keyDown(window, { key: 'Enter' });
    await Promise.resolve();
    expect(resolveCalls()).toHaveLength(0);
  });

  it('defers the queue on Escape and leaves the request pending', () => {
    queue(commitRequest);
    render(<ConfirmModal />);

    fireEvent.keyDown(window, { key: 'Escape' });
    expect(resolveCalls()).toHaveLength(0);
    expect(useStore.getState().pendingConfirmations).toHaveLength(1);
    expect(screen.queryByText('Git commit')).toBeNull();
  });

  it('resolves nothing on Enter while the queue is deferred', async () => {
    queue(commitRequest);
    render(<ConfirmModal />);
    fireEvent.keyDown(window, { key: 'Escape' });

    fireEvent.keyDown(window, { key: 'Enter' });
    await Promise.resolve();
    expect(resolveCalls()).toHaveLength(0);
  });

  it('rejects when Deny is pressed', async () => {
    queue(commitRequest);
    render(<ConfirmModal />);

    fireEvent.click(screen.getByText('Deny'));
    await waitFor(() => expect(lastResolve()).toMatchObject({ id: 'c1', approved: false }));
    await waitFor(() => expect(useStore.getState().pendingConfirmations).toHaveLength(0));
  });

  it('shows the next request once the first resolves', async () => {
    queue(commitRequest, closeRequest);
    render(<ConfirmModal />);
    expect(screen.getByText('+1 queued')).toBeTruthy();

    fireEvent.keyDown(window, { key: 'Enter' });
    await waitFor(() => expect(screen.getByText('Close MR')).toBeTruthy());
    expect(screen.queryByText('Git commit')).toBeNull();
    expect(screen.queryByText('+1 queued')).toBeNull();
  });

  it('reseeds the editable fields from the next request', async () => {
    queue(
      commitRequest,
      { ...commitRequest, id: 'c3', payload: { repo: 'mayo', branch: 'main', message: 'chore: bump' } },
    );
    render(<ConfirmModal />);

    fireEvent.keyDown(window, { key: 'Enter' });
    await waitFor(() => expect(screen.getByRole('textbox')).toHaveProperty('value', 'chore: bump'));
  });

  it('sends only the fields the user edited', async () => {
    queue(commitRequest);
    render(<ConfirmModal />);

    fireEvent.change(screen.getByRole('textbox'), { target: { value: 'fix: drop the stale worktree ref' } });
    fireEvent.keyDown(window, { key: 'Enter', ctrlKey: true });
    await waitFor(() =>
      expect(lastResolve()).toMatchObject({
        payloadOverrides: { message: 'fix: drop the stale worktree ref' },
      }),
    );
  });

  it('keeps the request pending and shows why when the resolve fails', async () => {
    invoked.mockRejectedValue('the bridge is gone' as never);
    queue(commitRequest);
    render(<ConfirmModal />);

    fireEvent.keyDown(window, { key: 'Enter' });
    await waitFor(() => expect(screen.getByText('the bridge is gone')).toBeTruthy());
    expect(useStore.getState().pendingConfirmations).toHaveLength(1);
  });

  it('blocks an MR with no title until one is typed', async () => {
    queue({ ...closeRequest, id: 'c4', op_type: OP.MR_CREATE, payload: { repo: 'mayo', branch: 'fix/parser' } });
    render(<ConfirmModal />);

    fireEvent.keyDown(window, { key: 'Enter', ctrlKey: true });
    await waitFor(() => expect(screen.getByText('Give the merge request a title first.')).toBeTruthy());
    expect(resolveCalls()).toHaveLength(0);

    fireEvent.change(screen.getAllByRole('textbox')[0], { target: { value: 'fix: drop the stale ref' } });
    fireEvent.keyDown(window, { key: 'Enter', ctrlKey: true });
    await waitFor(() => expect(lastResolve()).toMatchObject({ id: 'c4', approved: true }));
  });

  /** `focus()` on a disabled Approve is a no-op, and every shortcut needs focus inside. */
  it('keeps focus inside the dialog when Approve starts disabled', async () => {
    queue({ ...closeRequest, id: 'c5', op_type: OP.MR_CREATE, payload: { repo: 'mayo', branch: 'fix/parser' } });
    render(<ConfirmModal />);

    const dialog = await screen.findByRole('dialog');
    await waitFor(() => expect(dialog.contains(document.activeElement)).toBe(true));

    fireEvent.change(screen.getAllByRole('textbox')[0], { target: { value: 'fix: a title' } });
    // Plain Enter, with no click and no Tab: it only reaches `resolve` from inside the dialog.
    fireEvent.keyDown(window, { key: 'Enter' });
    await waitFor(() => expect(lastResolve()).toMatchObject({ id: 'c5', approved: true }));
  });
});
