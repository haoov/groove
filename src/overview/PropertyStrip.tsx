import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Check, ChevronDown } from 'lucide-react';
import { useStore } from '../shared/store';
import {
  AddField, MultiRow, Pill, hasValue, isHoursProperty,
  MULTI_KINDS, SINGLE_KINDS, type Row,
} from './propertyControls';
import type { PropertyValue, TaskSchema } from '../shared/ipc/ipc';

/**
 * The task's properties as a rail of controls, in the app's own badge language:
 * filled pills, uppercase, with a caret that is ALWAYS visible. A property is
 * something you change, so it has to look like something you can change — hiding
 * the affordance until hover was the mistake in the previous pass.
 *
 * Colour is reused, not invented: status takes the same colours as Home's queue
 * rows, priority the same tints as its `prio-badge`. Everything else stays neutral,
 * so the colour that IS there still means something.
 *
 * Single values sit in the rail; multi-value sets (components, tags) get a labelled
 * row beneath, because a pile of chips can't say which set it belongs to. Empty
 * properties live behind `+ field`. Every pill opens a popover — one interaction
 * model for select, status, date and number alike.
 */

export function PropertyStrip({
  notionPageId, hours, onHoursValue,
}: {
  notionPageId: string;
  hours?: React.ReactNode;
  onHoursValue?: (display: string) => void;
}) {
  const setLastError = useStore((s) => s.setLastError);
  // Which property IS priority comes from config, so its pill can take the
  // priority colours without inferring them from the value.
  const priorityProp = useStore((s) => s.config?.notion.properties.priority ?? null);
  const [schema, setSchema] = useState<TaskSchema | null>(null);
  const [values, setValues] = useState<PropertyValue[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  const [details, setDetails] = useState(false);
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

  const groups = useMemo(() => {
    const rows: Row[] = (schema?.properties ?? [])
      .filter((p) => p.kind !== 'title')
      .map((p) => ({ prop: p, current: values.find((v) => v.name === p.name) }));
    const editable = rows.filter((r) => r.prop.editable);
    const live = editable.filter((r) => hasValue(r.current) || revealed.has(r.prop.name));
    return {
      single: live.filter(
        (r) => SINGLE_KINDS.includes(r.prop.kind) && !isHoursProperty(r.prop.name, r.prop.kind),
      ),
      flags: live.filter((r) => r.prop.kind === 'checkbox'),
      multi: live.filter((r) => MULTI_KINDS.includes(r.prop.kind)),
      hoursRow: editable.find((r) => isHoursProperty(r.prop.name, r.prop.kind)),
      unset: editable.filter((r) => !hasValue(r.current) && !revealed.has(r.prop.name)),
      computed: rows.filter((r) => !r.prop.editable && r.current?.display),
    };
  }, [schema, values, revealed]);

  if (!schema) return null;

  return (
    <div className="props">
      <div className="props-rail">
        {groups.single.map((row) => (
          <Pill
            key={row.prop.name}
            row={row}
            busy={busy === row.prop.name}
            isPriority={row.prop.name === priorityProp}
            onChange={(v) => write(row.prop.name, v)}
          />
        ))}
        {groups.flags.map(({ prop, current }) => (
          <button
            key={prop.name}
            className={`ppill flag ${current?.value ? 'on' : ''}`}
            title={prop.name}
            onClick={() => write(prop.name, !current?.value)}
          >
            {current?.value === true && <Check size={11} strokeWidth={3} />}
            <span className="ppill-text">{prop.name}</span>
          </button>
        ))}
        {groups.unset.length > 0 && (
          <AddField
            fields={groups.unset.map((r) => r.prop.name)}
            onPick={(name) => setRevealed((s) => new Set(s).add(name))}
          />
        )}
      </div>

      {groups.multi.length > 0 && (
        <div className="props-sets">
          {groups.multi.map((row) => (
            <MultiRow
              key={row.prop.name}
              row={row}
              busy={busy === row.prop.name}
              onChange={(v) => write(row.prop.name, v)}
              onError={setLastError}
            />
          ))}
        </div>
      )}

      {hours && groups.hoursRow && hours}

      {groups.computed.length > 0 && (
        <div className="props-details">
          <button className="props-details-toggle" onClick={() => setDetails((v) => !v)}>
            {details ? 'hide' : 'details'} ({groups.computed.length})
            <ChevronDown size={10} strokeWidth={2.5} className={details ? 'flip' : undefined} />
          </button>
          {details &&
            groups.computed.map(({ prop, current }) => (
              <span className="props-detail" key={prop.name}>
                <span className="props-detail-label">{prop.name}</span>
                <span className="props-detail-value">{current?.display}</span>
              </span>
            ))}
        </div>
      )}
    </div>
  );
}
