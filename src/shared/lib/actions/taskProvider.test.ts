import { describe, expect, it } from 'vitest';
import { PROVIDER_IDS, configuredSources, providerCopy } from './taskProvider';
import type { Config } from '../../ipc/ipc';

const config = (over: Partial<Config> = {}) =>
  ({ notion: null, github: null, git: {}, ui: {}, ...over }) as Config;

describe('configuredSources', () => {
  it('keeps only the sources that are set up, in registry order', () => {
    expect(configuredSources(config())).toEqual([]);
    expect(configuredSources(config({ github: {} as never }))).toEqual(['github']);
    expect(configuredSources(config({ notion: {} as never, github: {} as never }))).toEqual(PROVIDER_IDS);
  });

  it('answers an empty list before the config loads', () => {
    expect(configuredSources(null)).toEqual([]);
    expect(configuredSources(undefined)).toEqual([]);
  });
});

describe('providerCopy', () => {
  it('names every provider id and falls back for an unknown one', () => {
    for (const id of PROVIDER_IDS) expect(providerCopy({ provider: id }).label).not.toBe('its source');
    expect(providerCopy({ provider: 'jira' }).label).toBe('its source');
    expect(providerCopy(null).label).toBe('its source');
  });
});
