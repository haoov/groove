import { useEffect, useMemo, useState, useCallback } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { useStore } from '../shared/store';
import {
  AddField, PropField, hasValue, type Row,
} from '../shared/ui/propertyControls';
import { TimeFields } from './TimeFields';
import type { PropertyValue, TaskSchema } from '../shared/ipc/ipc';

/**
 * The task's properties as a strip of labelled columns, each edited in a popover.
 * Empty editable properties live behind "+ field". Time is the last column.
 */

export function PropertyStrip({
  shortId, schema,
}: {
  shortId: string;
  schema: TaskSchema | null;
}) {
  const setLastError = useStore((s) => s.setLastError);
  const [values, setValues] = useState<PropertyValue[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  const [revealed, setRevealed] = useState<Set<string>>(new Set());

  const load = useCallback(() => {
    invoke<PropertyValue[]>('get_task_properties', { shortId })
      .then(setValues)
      .catch((e) => setLastError(String(e)));
  }, [shortId, setLastError]);

  useEffect(() => { load(); }, [load]);

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

  const hoursProperty = schema?.hours_property ?? null;
  const loggedDisplay = hoursProperty
    ? (values.find((v) => v.name === hoursProperty)?.display ?? '')
    : '';

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

        <TimeFields
          taskId={shortId}
          hoursProperty={hoursProperty}
          logged={loggedDisplay}
          onLogged={load}
        />
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
