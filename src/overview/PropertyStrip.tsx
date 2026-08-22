import { useEffect, useMemo, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { useStore } from '../shared/store';
import {
  AddField, PropField, hasValue, isHoursProperty, META_KINDS, type Row,
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
  notionPageId, onHoursValue, onHoursAvailable,
}: {
  notionPageId: string;
  onHoursValue?: (display: string) => void;
  /** Whether the task schema HAS an hours property. Hours render in the overview
   *  side column now, not here — the parent needs this to show that panel. */
  onHoursAvailable?: (available: boolean) => void;
}) {
  const setLastError = useStore((s) => s.setLastError);
  const [schema, setSchema] = useState<TaskSchema | null>(null);
  const [values, setValues] = useState<PropertyValue[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  const [revealed, setRevealed] = useState<Set<string>>(new Set());

  const load = () => {
    invoke<PropertyValue[]>('get_task_properties', { notionPageId })
      .then((vs) => {
        setValues(vs);
        const h = vs.find((v) => isHoursProperty(v.name, v.kind));
        if (h && onHoursValue) onHoursValue(h.display);
      })
      .catch((e) => setLastError(String(e)));
  };

  useEffect(() => {
    invoke<TaskSchema>('get_task_schema').then(setSchema).catch((e) => setLastError(String(e)));
  }, [setLastError]);

  useEffect(load, [notionPageId]);

  const write = async (name: string, value: unknown) => {
    setBusy(name);
    setValues((vs) => vs.map((v) => (v.name === name ? { ...v, value } : v)));
    try {
      await invoke<string>('update_task_property', { notionPageId, property: name, value });
    } catch (e) {
      setLastError(String(e));
      load();
    } finally {
      setBusy(null);
    }
  };

  const { shown, unset, hasHours } = useMemo(() => {
    const rows: Row[] = (schema?.properties ?? [])
      .filter((p) => !META_KINDS.includes(p.kind) && !isHoursProperty(p.name, p.kind))
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
    return {
      shown: shownRows,
      unset: unsetRows,
      hasHours: (schema?.properties ?? []).some((p) => p.editable && isHoursProperty(p.name, p.kind)),
    };
  }, [schema, values, revealed]);

  useEffect(() => { onHoursAvailable?.(hasHours); }, [hasHours]);

  if (!schema) return null;

  return (
    <div className="props">
      <div className="props-cols">
        {shown.map((row) => (
          <PropField
            key={row.prop.name}
            row={row}
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
