import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useStore } from '../store';
import { useRepoPicker, RepoPickerList, CloneRepoForm } from './repoPicker';

interface BranchSpec {
  repo_id: string;
  branch_name: string | null;
}

type WizardStep = 'repos' | 'branches' | 'confirming' | 'done';

export function TaskOpenWizard() {
  const { wizardTask, setWizardTask } = useStore();
  const [step, setStep] = useState<WizardStep>('repos');
  const [branchSpecs, setBranchSpecs] = useState<BranchSpec[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  // Shared repo picker: registering a repo seeds a branch spec; deselecting drops it.
  const {
    mainRepos, selectedRepos, setSelectedRepos,
    isSelected, isPending, toggleRepo, loadRepos,
  } = useRepoPicker({
    onSelect: (repo) => setBranchSpecs((p) => [...p, { repo_id: repo.id, branch_name: null }]),
    onDeselect: (repo) => setBranchSpecs((p) => p.filter((s) => s.repo_id !== repo.id)),
    onError: (msg) => setError(msg),
  });

  // Scan the clone pool when the wizard opens.
  useEffect(() => {
    if (!wizardTask) return;
    loadRepos();
  }, [wizardTask, loadRepos]);

  if (!wizardTask) return null;

  const close = () => {
    setWizardTask(null);
    setStep('repos');
    setSelectedRepos([]);
    setBranchSpecs([]);
    setError('');
  };

  const updateBranchSpec = (repoId: string, branchName: string) => {
    setBranchSpecs((prev) =>
      prev.map((s) => (s.repo_id === repoId ? { ...s, branch_name: branchName || null } : s))
    );
  };

  const provision = async () => {
    if (selectedRepos.length === 0) return;
    setLoading(true);
    setError('');
    const shortId = wizardTask.short_id;
    try {
      await invoke('set_task_repos', {
        shortId,
        repoIds: selectedRepos.map((r) => r.id),
      });
      await invoke('provision_worktrees', {
        taskId: shortId,
        branches: branchSpecs,
      });
      setStep('done');
      setTimeout(() => {
        close();
        invoke('open_task', { shortId }).catch(console.error);
      }, 800);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="wizard-overlay">
      <div className="wizard-modal">
        <div className="wizard-header">
          <div className="wizard-title">Open {wizardTask.short_id}</div>
          <div className="wizard-subtitle">{wizardTask.title}</div>
          <button className="wizard-close" onClick={close}>×</button>
        </div>

        <div className="wizard-steps">
          <span className={`wizard-step ${step === 'repos' ? 'active' : 'done'}`}>1. Repos</span>
          <span className="wizard-step-sep">›</span>
          <span className={`wizard-step ${step === 'branches' ? 'active' : ''}`}>2. Branches</span>
          <span className="wizard-step-sep">›</span>
          <span className={`wizard-step ${step === 'confirming' || step === 'done' ? 'active' : ''}`}>
            3. Provision
          </span>
        </div>

        {/* Step 1: Pick repos */}
        {step === 'repos' && (
          <div className="wizard-body">
            <p className="wizard-desc">Select the repositories needed for this task.</p>

            {mainRepos.length === 0 ? (
              <p className="wizard-empty">No repos in the pool yet — clone one below.</p>
            ) : (
              <RepoPickerList
                repos={mainRepos}
                isSelected={isSelected}
                isPending={isPending}
                onToggle={toggleRepo}
              />
            )}

            <CloneRepoForm onCloned={(repo) => { loadRepos(); toggleRepo(repo); }} />

            {error && <div className="wizard-error">{error}</div>}

            <div className="wizard-footer">
              <button className="btn-secondary" onClick={close}>Cancel</button>
              <button
                className="btn-primary"
                onClick={() => setStep('branches')}
                disabled={selectedRepos.length === 0}
              >
                Next →
              </button>
            </div>
          </div>
        )}

        {/* Step 2: Branch names */}
        {step === 'branches' && (
          <div className="wizard-body">
            <p className="wizard-desc">
              Confirm or edit the branch names.
            </p>
            <div className="wizard-branch-list">
              {selectedRepos.map((repo) => {
                const spec = branchSpecs.find((s) => s.repo_id === repo.id);
                return (
                  <div key={repo.id} className="wizard-branch-item">
                    <span className="wizard-branch-repo">{repo.project}</span>
                    {/* Real text, not a hint: the point is to prepend or append to
                        the task id without retyping it. */}
                    <input
                      className="wizard-input"
                      placeholder={wizardTask.short_id.toLowerCase()}
                      value={spec?.branch_name ?? wizardTask.short_id.toLowerCase()}
                      onChange={(e) => updateBranchSpec(repo.id, e.target.value)}
                    />
                  </div>
                );
              })}
            </div>
            {error && <div className="wizard-error">{error}</div>}
            <div className="wizard-footer">
              <button className="btn-secondary" onClick={() => setStep('repos')}>← Back</button>
              <button className="btn-primary" onClick={() => setStep('confirming')}>Next →</button>
            </div>
          </div>
        )}

        {/* Step 3: Confirm */}
        {step === 'confirming' && (
          <div className="wizard-body">
            <p className="wizard-desc">The following worktrees will be created:</p>
            <div className="wizard-confirm-list">
              {selectedRepos.map((repo) => {
                const spec = branchSpecs.find((s) => s.repo_id === repo.id);
                const branch = spec?.branch_name ?? wizardTask.short_id.toLowerCase();
                return (
                  <div key={repo.id} className="wizard-confirm-item">
                    <span className="wizard-confirm-repo">{repo.project}</span>
                    <span className="wizard-confirm-branch">→ {branch}</span>
                  </div>
                );
              })}
            </div>
            {error && <div className="wizard-error">{error}</div>}
            <div className="wizard-footer">
              <button className="btn-secondary" onClick={() => setStep('branches')}>← Back</button>
              <button className="btn-primary" onClick={provision} disabled={loading}>
                {loading ? 'Provisioning…' : 'Provision worktrees'}
              </button>
            </div>
          </div>
        )}

        {step === 'done' && (
          <div className="wizard-body wizard-done">
            <div className="wizard-done-icon">✓</div>
            <p>Worktrees created. Opening workspace…</p>
          </div>
        )}
      </div>
    </div>
  );
}
