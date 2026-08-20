import { useEffect, useState } from 'react';
import { call } from '../../shared/ipc/client';

/** The bottom status line. The MCP endpoint is a real read; sync/branch fill in
 *  with the sessions + git slices. */
export function StatusBar() {
  const [endpoint, setEndpoint] = useState('…');
  useEffect(() => {
    call<string>('get_mcp_endpoint').then(setEndpoint).catch(() => setEndpoint('unavailable'));
  }, []);
  return (
    <footer className="statusbar">
      <div className="spring" />
      <span>sync <b>idle</b></span>
      <span>mcp <b>{endpoint}</b></span>
    </footer>
  );
}
