import { useEffect, useState } from 'react';
import { Plus, Trash2, Lock, Copy } from 'lucide-react';
import { invoke } from '../shared/ipc/invoke';
import { useStore } from '../shared/store';
import type { AgentSkill } from '../shared/ipc/ipc';
import './actions.css';

/** The agent's actions in Settings. The editor is the raw SKILL.md — the same thing
 *  on disk. Core skills are read-only. */

/** What a new action starts as. */
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
  // Editing here changes the same files the agents loaded at launch.
  const setSkillsStale = useStore((s) => s.setSkillsStale);

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

  // A copy with no name yet, so saving cannot overwrite the core skill.
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
      setSkillsStale(true);
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
      setSkillsStale(true);
      setEditing(null);
      setReport(null);
    } catch (e) {
      setLastError(String(e));
    }
  };

  // The hint, not the description — the description is written for the model.
  const row = (s: AgentSkill) => (
    <button key={s.id} className="skill-row" onClick={() => void open(s)} title={s.hint}>
      {!s.editable && <Lock size={12} strokeWidth={1.75} className="skill-lock" />}
      <span className="skill-id">{s.id}</span>
      <span className="skill-desc">{s.hint}</span>
    </button>
  );

  return (
    <>
      <div className="settings-section-title">Actions</div>

      <div className="skill-group">
        <div className="skill-group-head">
          <span className="skill-group-label">Core</span>
          <span className="skill-group-note">Read-only. Open one to copy it as a starting point.</span>
        </div>
        <div className="skill-list">{core.map(row)}</div>
      </div>

      <div className="skill-group">
        <div className="skill-group-head">
          <span className="skill-group-label">User</span>
          <span className="skill-group-note">Editable, and kept across app updates.</span>
        </div>
        {mine.length > 0 ? (
          <div className="skill-list">{mine.map(row)}</div>
        ) : (
          <p className="skill-empty">Nothing here yet.</p>
        )}
        <button className="btn-secondary skill-new" onClick={create}>
          <Plus size={12} strokeWidth={2} /> New action
        </button>
      </div>

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
