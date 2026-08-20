import { useEffect } from 'react';
import { useStore } from '../shared/store';
import { LiveSection } from './LiveSection';
import { UpNext } from './UpNext';

export function Home() {
  const refreshHome = useStore((s) => s.refreshHome);
  const refreshTasks = useStore((s) => s.refreshTasks);
  const refreshQueue = useStore((s) => s.refreshQueue);

  useEffect(() => {
    void refreshHome();
    void refreshTasks();
    void refreshQueue();
  }, [refreshHome, refreshTasks, refreshQueue]);

  return (
    <div className="home">
      <div className="home-inner">
        <LiveSection />
        <UpNext />
      </div>
      {/* Scratch terminal — a deskless shell; wired in the terminal slice. */}
      <div className="scratch" title="Scratch shell — arrives in the terminal slice">
        <span className="lbl">scratch</span>
        <span className="pr">~/worktrees ❯</span>
      </div>
    </div>
  );
}
