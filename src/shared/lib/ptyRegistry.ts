// PTY output routing: one xterm handler per PTY session. Output that arrives before a handler
// mounts is buffered, capped, and flushed on registration.

const ptyOutputHandlers = new Map<string, (data: Uint8Array) => void>();
const MAX_PTY_BUFFER_CHUNKS = 256;
const ptyOutputBuffers = new Map<string, Uint8Array[]>();

/** Base64 to bytes, once, on the way to xterm. */
export function decodeChunk(b64: string): Uint8Array {
  const text = atob(b64);
  const bytes = new Uint8Array(text.length);
  for (let i = 0; i < text.length; i++) bytes[i] = text.charCodeAt(i);
  return bytes;
}

/** Bytes to base64 for `write_pty`. Chunked: String.fromCharCode has an argument limit. */
export function bytesToB64(bytes: Uint8Array): string {
  let bin = '';
  const CHUNK = 0x8000;
  for (let i = 0; i < bytes.length; i += CHUNK) {
    bin += String.fromCharCode(...bytes.subarray(i, i + CHUNK));
  }
  return btoa(bin);
}

export function registerPtyHandler(sessionId: string, handler: (data: Uint8Array) => void) {
  ptyOutputHandlers.set(sessionId, handler);
  // Flush what arrived before the handler mounted, in order.
  const buffered = ptyOutputBuffers.get(sessionId);
  if (buffered) {
    ptyOutputBuffers.delete(sessionId);
    for (const chunk of buffered) handler(chunk);
  }
}

export function unregisterPtyHandler(sessionId: string) {
  ptyOutputHandlers.delete(sessionId);
  ptyOutputBuffers.delete(sessionId);
}

/** Route one pty_output chunk: straight to the handler, or into the buffer. */
export function deliverPtyOutput(sessionId: string, b64: string) {
  const data = decodeChunk(b64);
  const handler = ptyOutputHandlers.get(sessionId);
  if (handler) {
    handler(data);
    return;
  }
  let buf = ptyOutputBuffers.get(sessionId);
  if (!buf) { buf = []; ptyOutputBuffers.set(sessionId, buf); }
  buf.push(data);
  if (buf.length > MAX_PTY_BUFFER_CHUNKS) buf.shift();
}
