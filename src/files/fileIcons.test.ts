import { describe, expect, it } from 'vitest';
import { brandFor } from './fileIcons';

const title = (name: string) => brandFor(name)?.title ?? null;

describe('brandFor', () => {
  it('matches on extension', () => {
    expect(title('src/main.rs')).toBe('Rust');
    expect(title('app.tsx')).toBe('React');
    expect(title('script.py')).toBe('Python');
  });

  it('matches extensionless and dotfile names', () => {
    expect(title('Dockerfile')).toBe('Docker');
    expect(title('.env')).toBe('.ENV');
    expect(title('.gitignore')).toBe('Git');
  });

  it('prefers a whole-name match over the extension', () => {
    expect(title('charts/api/Chart.yaml')).toBe('Helm');
    expect(title('values.yaml')).toBe('Helm');
    expect(title('_helpers.tpl')).toBe('Helm');
    expect(title('config/settings.yaml')).toBe('YAML');
  });

  it('ignores case', () => {
    expect(title('MAIN.RS')).toBe('Rust');
    expect(title('dockerfile')).toBe('Docker');
  });

  it('returns null with nothing to show', () => {
    expect(brandFor('pnpm-lock.yaml')).not.toBeNull();
    expect(brandFor('notes.txt')).toBeNull();
    expect(brandFor('LICENSE')).toBeNull();
  });
});
