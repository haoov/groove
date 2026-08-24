/** A pasted Notion id has to keep working: the people picker is a filterable text
 *  input, and one that cannot find you must not be a dead end. */
export function looksLikeNotionId(text: string): boolean {
  const t = text.trim().toLowerCase();
  return /^[0-9a-f]{32}$/.test(t) || /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/.test(t);
}
