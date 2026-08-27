import { useEffect, useState } from 'react';
import { Plus, Trash2, Lock, Copy } from 'lucide-react';
import { invoke } from '../shared/ipc/invoke';
import { useStore } from '../shared/store';
import type { AgentSkill } from '../shared/ipc/ipc';
import './actions.css';

/**
 * The agent's actions, as a Settings section.
 *
 * A skill is a `SKILL.md` the agent reads at the moment it is invoked, so the
 * editor is deliberately the raw file: front matter and prose, the same thing on
 * disk. Anything friendlier would be a second format to keep in sync with a
 * format Claude Code already defines.
 *
 * Core skills are read-only and open as the starting point for the user's own —
 * a working example beats an empty file.
 */

/** What a new action starts as. The two `groove-*` keys are Groove's, and unknown
 *  front-matter keys are passed through by Claude Code untouched. */
const TEMPLATE = `---
name: NAME
description: What this does, then "Use when …" with the words you would type to ask for it. Claude Code matches this to invoke the skill on its own.
groove-kinds: task
groove-label: NAME
groove-hint: One line. Groove shows it beside the action.
---

# What to do

1. First step.
2. Second step.
`;

type Editing = { name: string; previous: string | null; body: string };

export function SkillsSection() {
  const skills = useStore((s) => s.skills);
  const loadSkills = useStore((s) => s.loadSkills);
  const setLastError = useStore((s) => s.setLastError);

  const [editing, setEditing] = useState<Editing | null>(null);
  const [viewing, setViewing] = useState<AgentSkill | null>(null);
  const [body, setBody] = useState('');
  const [report, setReport] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => { void loadSkills(); }, [loadSkills]);

  const mine = skills.filter((s) => s.editable);
  const core = skills.filter((s) => !s.editable);

  const open = async (skill: AgentSkill) => {
    setReport(null);
    try {
      const text = await invoke<string>('read_agent_skill', { id: skill.id });
      if (skill.editable) {
        setViewing(null);
        setEditing({ name: skill.name, previous: skill.name, body: text });
      } else {
        setEditing(null);
        setViewing(skill);
        setBody(text);
      }
    } catch (e) {
      setLastError(String(e));
    }
  };

  const create = () => {
    setViewing(null);
    setReport(null);
    setEditing({ name: '', previous: null, body: TEMPLATE });
  };

  // A core skill copied into the user's own: same body, no name yet, so saving
  // cannot overwrite the original.
  const copyToMine = () => {
    setViewing(null);
    setReport(null);
    setEditing({ name: '', previous: null, body });
  };

  const save = async () => {
    if (!editing) return;
    setSaving(true);
    try {
      const out = await invoke<string | null>('save_user_skill', {
        name: editing.name,
        body: editing.body,
        previousName: editing.previous,
      });
      setReport(out);
      await loadSkills();
      setEditing({ ...editing, previous: editing.name });
    } catch (e) {
      setLastError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const remove = async () => {
    if (!editing?.previous) return;
    try {
      await invoke('delete_user_skill', { name: editing.previous });
      await loadSkills();
      setEditing(null);
      setReport(null);
    } catch (e) {
      setLastError(String(e));
    }
  };

  return (
    <>
      <div className="settings-section-title">Agent actions</div>

      <div className="skill-list">
        {/* The hint, not the description: the description is written for the model
            to match the user's words against, and it is a paragraph. */}
        {core.map((s) => (
          <button key={s.id} className="skill-row" onClick={() => void open(s)} title={s.hint}>
            <Lock size={12} strokeWidth={1.75} className="skill-lock" />
            <span className="skill-id">{s.id}</span>
            <span className="skill-desc">{s.hint}</span>
          </button>
        ))}
        {mine.map((s) => (
          <button key={s.id} className="skill-row mine" onClick={() => void open(s)} title={s.hint}>
            <span className="skill-id">{s.id}</span>
            <span className="skill-desc">{s.hint}</span>
          </button>
        ))}
      </div>

      <button className="btn-secondary skill-new" onClick={create}>
        <Plus size={12} strokeWidth={2} /> New action
      </button>

      {viewing && (
        <div className="skill-editor">
          <div className="skill-editor-head">
            <span className="skill-id">{viewing.id}</span>
            <span className="skill-readonly">read-only</span>
            <button className="btn-secondary" onClick={copyToMine}>
              <Copy size={11} strokeWidth={2} /> Copy to my actions
            </button>
            <button className="btn-secondary" onClick={() => setViewing(null)}>Close</button>
          </div>
          <textarea className="skill-body" value={body} readOnly spellCheck={false} />
        </div>
      )}

      {editing && (
        <div className="skill-editor">
          <div className="skill-editor-head">
            <span className="skill-prefix">user:</span>
            <input
              className="settings-input skill-name"
              value={editing.name}
              placeholder="deploy-check"
              spellCheck={false}
              onChange={(e) => setEditing({ ...editing, name: e.target.value })}
            />
            <button className="btn-secondary" disabled={!editing.name || saving} onClick={() => void save()}>
              Save
            </button>
            {editing.previous && (
              <button className="btn-secondary danger" onClick={() => void remove()}>
                <Trash2 size={11} strokeWidth={2} /> Delete
              </button>
            )}
            <button className="btn-secondary" onClick={() => { setEditing(null); setReport(null); }}>
              Cancel
            </button>
          </div>
          <textarea
            className="skill-body"
            value={editing.body}
            spellCheck={false}
            onChange={(e) => setEditing({ ...editing, body: e.target.value })}
          />
          {/* `claude plugin validate` reads the file the agent will read, so its
              warnings are the real ones — a missing description, no front matter. */}
          {report && <pre className="skill-report">{report}</pre>}
        </div>
      )}
    </>
  );
}
