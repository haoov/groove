import { useEffect, useMemo, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { useStore } from '../shared/store';
import {
  AddField, PropField, hasValue, type Row,
} from '../shared/ui/propertyControls';
import type { PropertyValue, TaskSchema } from '../shared/ipc/ipc';

/**
 * The task's properties as a framed strip of labelled columns (the mockup): each
 * property is a small uppercase key over its value, divided by hairlines, with
 * the "+ field" button pinned to the right. Every value opens a popover to edit —
 * one interaction model for status, select, date, number and multi-value sets.
 *
 * Shown = properties that have a value (or were just revealed via + field), plus
 * read-only computed fields; empty editable properties live behind + field.
 * Hours is not here — it has its own block in the overview side column.
 */

export function PropertyStrip({
  shortId, schema, onHoursValue,
}: {
  shortId: string;
  schema: TaskSchema | null;
  onHoursValue?: (display: string) => void;
}) {
  const setLastError = useStore((s) => s.setLastError);
  const [values, setValues] = useState<PropertyValue[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  const [revealed, setRevealed] = useState<Set<string>>(new Set());

  const load = () => {
    invoke<PropertyValue[]>('get_task_properties', { shortId })
      .then((vs) => {
        setValues(vs);
        const h = schema?.hours_property && vs.find((v) => v.name === schema.hours_property);
        if (h && onHoursValue) onHoursValue(h.display);
      })
      .catch((e) => setLastError(String(e)));
  };

  useEffect(load, [shortId]);

  const write = async (name: string, value: unknown) => {
    setBusy(name);
    setValues((vs) => vs.map((v) => (v.name === name ? { ...v, value } : v)));
    try {
      await invoke<string>('update_task_property', { shortId, property: name, value });
    } catch (e) {
      setLastError(String(e));
      load();
    } finally {
      setBusy(null);
    }
  };

  const { shown, unset } = useMemo(() => {
    const rows: Row[] = (schema?.properties ?? [])
      .filter((p) => !p.meta && p.name !== schema?.hours_property)
      .map((p) => ({ prop: p, current: values.find((v) => v.name === p.name) }));
    // Columns, in schema order: editable props that carry a value (or were just
    // revealed), plus read-only computed props that have something to show.
    const shownRows = rows.filter((r) =>
      r.prop.editable
        ? (hasValue(r.current) || revealed.has(r.prop.name))
        : !!r.current?.display,
    );
    const unsetRows = rows.filter(
      (r) => r.prop.editable && !hasValue(r.current) && !revealed.has(r.prop.name),
    );
    return { shown: shownRows, unset: unsetRows };
  }, [schema, values, revealed]);

  if (!schema) return null;

  return (
    <div className="props">
      <div className="props-cols">
        {shown.map((row) => (
          <PropField
            key={row.prop.name}
            row={row}
            shortId={shortId}
            busy={busy === row.prop.name}
            onChange={(v) => write(row.prop.name, v)}
            onError={setLastError}
          />
        ))}
      </div>
      {unset.length > 0 && (
        <div className="props-add">
          <AddField
            fields={unset.map((r) => r.prop.name)}
            onPick={(name) => setRevealed((s) => new Set(s).add(name))}
          />
        </div>
      )}
    </div>
  );
}
