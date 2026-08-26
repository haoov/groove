import { useStore } from '../shared/store';
import { SkillsSection } from '../actions/SkillsSection';

/** What the agent can be asked to do. Groove's own actions are read-only; the
 *  user's live beside them and open in the same editor. */
export function ActionsPanel() {
  const suggestActions = useStore((s) => s.config?.ui.suggest_actions ?? true);
  const setSuggestActions = useStore((s) => s.setSuggestActions);

  return (
    <section className="settings-section">
      <SkillsSection />
      <label className="settings-toggle-row">
        <span className="settings-toggle-text">
          <span className="settings-toggle-label">Suggest actions</span>
          <span className="settings-toggle-hint">
            Offer an action where one fits, such as scaffolding a task with no repos.
            Off hides the offers; every action stays a button and a slash command.
          </span>
        </span>
        <button
          role="switch"
          aria-checked={suggestActions}
          className={`settings-switch ${suggestActions ? 'on' : ''}`}
          onClick={() => setSuggestActions(!suggestActions)}
        >
          <span className="settings-switch-knob" />
        </button>
      </label>
    </section>
  );
}
