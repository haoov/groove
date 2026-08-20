import { useStore } from '../../shared/store';
import { Icon } from '../../shared/ui';

/** The activity rail — the app spine. Home + Reviews are global; the session
 *  panel buttons (Overview/Files/Git/Notes) arrive with the session slices. */
export function Rail() {
  const view = useStore((s) => s.view);
  const setView = useStore((s) => s.setView);

  return (
    <nav className="rail">
      <div className="rgroup">
        <button className={`r${view === 'home' ? ' on' : ''}`} title="Home" onClick={() => setView('home')}>
          <Icon name="home" />
        </button>
        <button className="r" title="Reviews">
          <Icon name="reviews" />
          <i className="num">3</i>
        </button>
      </div>
      <button className="r r-bottom" title="Command palette"><Icon name="cmd" /></button>
    </nav>
  );
}
