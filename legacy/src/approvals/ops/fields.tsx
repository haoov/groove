interface FieldProps { label: string; value: string; mono?: boolean; block?: boolean }

export function Field({ label, value, mono = false, block = false }: FieldProps) {
  return (
    <div className={`cp-field ${block ? 'cp-field--block' : ''}`}>
      <span className="cp-field-label">{label}</span>
      <span className={`cp-field-value ${mono ? 'cp-field-value--mono' : ''}`}>{value}</span>
    </div>
  );
}

interface EditableFieldProps extends Omit<FieldProps, 'mono' | 'block'> {
  onChange: (v: string) => void;
  multiline?: boolean;
  placeholder?: string;
}

export function EditableField({ label, value, onChange, multiline = false, placeholder }: EditableFieldProps) {
  return (
    <div className="cp-field cp-field--block">
      <span className="cp-field-label">{label}</span>
      {multiline ? (
        <textarea
          className="cp-field-input cp-field-textarea"
          value={value}
          placeholder={placeholder}
          onChange={(e) => onChange(e.target.value)}
        />
      ) : (
        <input
          className="cp-field-input"
          value={value}
          placeholder={placeholder}
          onChange={(e) => onChange(e.target.value)}
        />
      )}
    </div>
  );
}
