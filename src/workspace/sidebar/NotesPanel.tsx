import { invoke } from '../../shared/ipc/invoke';
import { useSession, useStore } from '../../shared/store';
import type { Worktree } from '../../shared/ipc/ipc';
import { AnnotationsTab } from '../../notes/AnnotationsTab';
import { MrThreadsSection } from '../../notes/MrThreads';

/** Notes: the local annotations plus the MR discussion when one exists. */
export function NotesPanel({
  repoId, worktreeForRepo,
}: {
  repoId: string;
  worktreeForRepo: (id: string) => Worktree | undefined;
}) {
  const activeRepos = useSession((s) => s.activeRepos);
  const annotations = useSession((s) => s.annotations);
  const mrs = useSession((s) => s.mrs);
  const mrThreadsByRepo = useSession((s) => s.mrThreadsByRepo);
  const openTab = useSession((s) => s.openTab);
  const resolveAnnotation = useSession((s) => s.resolveAnnotation);
  const removeAnnotation = useSession((s) => s.removeAnnotation);
  const updateAnnotation = useSession((s) => s.updateAnnotation);
  const bumpMrs = useSession((s) => s.bumpMrs);
  const notify = useStore((s) => s.notify);
  const setLastError = useStore((s) => s.setLastError);

  const wt = worktreeForRepo(repoId);
  const repoMr = mrs.find((m) => m.worktree_id === wt?.id) ?? null;

  return (
    <>
      <AnnotationsTab
        annotations={annotations.filter((a) => a.repo_id === repoId)}
        repoFor={(id) => activeRepos.find((r) => r.id === id)}
        onResolve={async (id) => {
          try {
            await invoke('resolve_annotation', { id });
            resolveAnnotation(id);
          } catch (e) {
            setLastError(e);
          }
        }}
        onDelete={async (id) => {
          try {
            await invoke('delete_annotation', { id });
            removeAnnotation(id);
          } catch (e) {
            setLastError(e);
          }
        }}
        onEdit={async (id, content) => {
          try {
            await invoke('update_annotation', { id, content });
            updateAnnotation(id, content);
          } catch (e) {
            setLastError(e);
          }
        }}
        // Open the file at the annotated line (the editor takes cursorLine).
        onOpen={(a) =>
          openTab({
            repoId: a.repo_id,
            filePath: a.file_path,
            view: 'edit',
            cursorLine: a.start_line,
          })
        }
        mr={repoMr}
        onPostToMr={async (a) => {
          try {
            await invoke('post_mr_comment', {
              mrId: repoMr!.id,
              body: a.content,
              filePath: a.file_path,
              line: a.start_line,
            });
            await invoke('resolve_annotation', { id: a.id });
            resolveAnnotation(a.id);
            bumpMrs();
            notify({ kind: 'success', source: 'mr', taskId: a.session_id, title: `Comment posted on ${a.file_path.split('/').pop()}:${a.start_line}` });
          } catch (e) {
            setLastError(e);
          }
        }}
      />
      {repoMr && (
        <div className="notes-threads">
          <div className="notes-threads-title">MR discussion</div>
          <MrThreadsSection
            threads={mrThreadsByRepo[repoId] ?? []}
            mr={repoMr}
            onResolved={bumpMrs}
          />
        </div>
      )}
    </>
  );
}
