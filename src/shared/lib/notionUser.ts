/** True for either spelling of a Notion id. */
export function looksLikeNotionId(text: string): boolean {
  const t = text.trim().toLowerCase();
  return /^[0-9a-f]{32}$/.test(t) || /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/.test(t);
}
