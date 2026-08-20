import type { StateCreator } from 'zustand';
import { call } from '../shared/ipc/client';
import type { Store } from '../shared/store/types';

/** A pending confirmation, mirrored from the `confirmation_requested` event
 *  (see approvals/bridge.rs — the payload is inline JSON, not a ts-rs type). */
export interface Confirmation {
  id: string;
  session_id: string | null;
  op_type: string;
  payload: Record<string, unknown>;
  origin: string;
}

export interface ApprovalsSlice {
  confirmations: Confirmation[];
  pushConfirmation: (c: Confirmation) => void;
  dropConfirmation: (id: string) => void;
  resolveConfirmation: (id: string, approved: boolean, overrides?: Record<string, unknown>) => Promise<void>;
}

export const approvalsSlice: StateCreator<Store, [], [], ApprovalsSlice> = (set, get) => ({
  confirmations: [],

  pushConfirmation: (c) =>
    set((st) =>
      st.confirmations.some((x) => x.id === c.id)
        ? st
        : { confirmations: [...st.confirmations, c] },
    ),

  dropConfirmation: (id) =>
    set((st) => ({ confirmations: st.confirmations.filter((c) => c.id !== id) })),

  resolveConfirmation: async (id, approved, overrides) => {
    // Drop it now so the modal advances at once; `confirmation_resolved` is the
    // backstop that removes it if the call is slow or races another decision.
    get().dropConfirmation(id);
    try {
      await call('resolve_confirmation', { id, approved, payloadOverrides: overrides ?? null });
    } catch (e) {
      console.warn('resolve_confirmation failed', e);
    }
  },
});
