import type { ProviderId } from '../ipc/ipc';

/** Copy that names a task's source. Only copy; capabilities come from the schema. */
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

// Keyed by ProviderId: a provider added on the Rust side fails the build here until its copy exists.
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

/** Copy for a task's provider. An unknown provider gets neutral copy. */
export function providerCopy(
  task: { provider?: string | null } | null | undefined,
): ProviderCopy {
  const id = task?.provider;
  return (id && (PROVIDERS as Record<string, ProviderCopy>)[id]) || FALLBACK;
}
