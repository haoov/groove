import type { DetectedSchema } from '../../shared/ipc/ipc';

/** What the app read off a task source, shown before it is saved. */
export function DetectedPanel({ detected, note }: { detected: DetectedSchema; note: string }) {
  return (
    <div className="firstrun-detected">
      <div className="firstrun-detected-head">What Groove read</div>
      <dl className="firstrun-detected-list">
        <dt>Title</dt><dd>{detected.title_property}</dd>
        <dt>Status</dt><dd>{detected.status_property}</dd>
        <dt>Priority</dt><dd>{detected.priority_property ?? <em>none</em>}</dd>
        <dt>Sprint</dt><dd>{detected.sprint_property ?? <em>none</em>}</dd>
        <dt>Project</dt><dd>{detected.project_property ?? <em>none</em>}</dd>
        <dt>Assignee</dt><dd>{detected.assignee_property ?? <em>none</em>}</dd>
      </dl>
      <div className="firstrun-detected-head">Status values Groove will set</div>
      <dl className="firstrun-detected-list">
        <dt>Filing a task</dt><dd>{detected.status_ready || <em>not found</em>}</dd>
        <dt>Picking it up</dt><dd>{detected.status_in_progress || <em>not found</em>}</dd>
        <dt>Finishing it</dt><dd>{detected.status_done || <em>not found</em>}</dd>
      </dl>
      <span className="firstrun-hint">
        {note} Every value is written to the config file and can be corrected there.
        {detected.status_options.length > 0 && (
          <> All options: {detected.status_options.join(' · ')}.</>
        )}
      </span>
    </div>
  );
}
