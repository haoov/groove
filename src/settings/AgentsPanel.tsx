import { useStore } from '../shared/store';
import { SkillsSection } from '../actions/SkillsSection';

/** Everything about the agent. Actions today; the rest of its configuration
 *  belongs here as it arrives, rather than in a panel of its own. */
export function AgentsPanel() {
  const suggestActions = useStore((s) => s.config?.ui.suggest_actions ?? true);
  const setSuggestActions = useStore((s) => s.setSuggestActions);

  return (
    <>
      <section className="settings-section">
        <SkillsSection />
      </section>

      <section className="settings-section">
        <div className="settings-section-title">Suggestions</div>
        <label className="settings-toggle-row">
          <span className="settings-toggle-text">
            <span className="settings-toggle-label">Offer an action where one fits</span>
            <span className="settings-toggle-hint">
              Such as reading a task that has no repos yet. Off hides the offers and
              nothing else: every action stays in the menu and a slash command.
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
    </>
  );
}
