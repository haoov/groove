import type { ProviderId } from '../ipc/ipc';

/** Copy that names a specific task's source. Only copy — anything a provider can
 *  or cannot do is answered by its schema, not by a flag here. */
interface ProviderCopy {
  label: string;
  /** What one task is called there. */
  item: string;
  /** Warning shown before replacing a body, or null when nothing can be lost. */
  bodyWarning: string | null;
  /** What finishing does at the source. */
  finish: string;
  /** What discarding does at the source. */
  discard: string;
}

// Keyed by ProviderId, not by string: a provider added on the Rust side then
// fails the build here until its copy exists, instead of silently reading
// FALLBACK for the rest of the app's life.
const PROVIDERS: Record<ProviderId, ProviderCopy> = {
  notion: {
    label: 'Notion',
    item: 'page',
    bodyWarning: 'Replaces the whole page body. Blocks markdown cannot represent are lost.',
    finish: 'The task is marked Done in Notion.',
    discard: 'The Notion page goes to your workspace trash, where it can be restored for 30 days.',
  },
  github: {
    label: 'GitHub',
    item: 'issue',
    // An issue body is markdown already, so a round trip loses nothing.
    bodyWarning: null,
    finish: "The board's Status is set to done.",
    discard: 'The issue is closed as not planned.',
  },
};

const FALLBACK: ProviderCopy = {
  label: 'its source',
  item: 'task',
  bodyWarning: null,
  finish: 'The task is marked done at its source.',
  discard: 'The task is closed at its source.',
};

/** The wire carries `provider` as a plain string, so a value the app does not
 *  know still has to render — as neutral copy, never as a wrong provider. */
export function providerCopy(
  task: { provider?: string | null } | null | undefined,
): ProviderCopy {
  const id = task?.provider;
  return (id && (PROVIDERS as Record<string, ProviderCopy>)[id]) || FALLBACK;
}
