import s from './ui.module.css';

export interface TextBoxProps {
  value: string;
  minHeight?: number;
  mono?: boolean;
  disabled?: boolean;
  placeholder?: string;
  onChange?: (value: string) => void;
}

/** Multi-line text: a graph description, a system prompt, a note. */
export function TextBox({
  value,
  minHeight = 64,
  mono = false,
  disabled = false,
  placeholder,
  onChange,
}: TextBoxProps) {
  return (
    <textarea
      className={`${s.textBox}${mono ? ` ${s.textBoxMono}` : ''}`}
      style={{ minHeight }}
      value={value}
      disabled={disabled}
      readOnly={!onChange}
      placeholder={placeholder}
      onChange={(e) => onChange?.(e.target.value)}
    />
  );
}
