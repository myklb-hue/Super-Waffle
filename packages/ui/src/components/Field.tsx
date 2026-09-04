import { Icon } from './Icon';
import type { IconName } from './icons';
import s from './ui.module.css';

export interface FieldProps {
  value: string;
  icon?: IconName;
  mono?: boolean;
  muted?: boolean;
  /** Renders as a select: a chevron, and onOpen instead of onChange. */
  select?: boolean;
  suffix?: string;
  disabled?: boolean;
  /** Shows the value's own skeleton rather than collapsing the row. */
  loading?: boolean;
  /** A message under the field; the border turns red. */
  error?: string;
  placeholder?: string;
  onChange?: (value: string) => void;
  onOpen?: () => void;
}

/** One value in the inspector. Editable, a select, or static when neither
 *  onChange nor onOpen is given. */
export function Field({
  value,
  icon,
  mono = false,
  muted = false,
  select = false,
  suffix,
  disabled = false,
  loading = false,
  error,
  placeholder,
  onChange,
  onOpen,
}: FieldProps) {
  const cls = [
    s.field,
    mono && s.fieldMono,
    muted && s.fieldMuted,
    disabled && s.fieldDisabled,
    loading && s.fieldLoading,
    error && s.fieldError,
  ]
    .filter(Boolean)
    .join(' ');

  const inner = (
    <>
      {icon && <Icon name={icon} size={12} color="text-low" />}
      {loading ? (
        <span className={s.skeleton} />
      ) : onChange && !select ? (
        <input
          className={s.fieldInput}
          value={value}
          placeholder={placeholder}
          disabled={disabled}
          aria-invalid={error ? true : undefined}
          onChange={(e) => onChange(e.target.value)}
        />
      ) : (
        <span className={s.fieldStatic}>{value || placeholder}</span>
      )}
      {suffix && !loading && <span className={s.fieldSuffix}>{suffix}</span>}
      {select && (
        <span className={s.fieldChevron}>
          <Icon name="chev" size={10} color="text-low" strokeWidth={2} />
        </span>
      )}
    </>
  );

  return (
    <span className={s.fieldWrap}>
      {select ? (
        <button type="button" className={cls} disabled={disabled} onClick={onOpen}>
          {inner}
        </button>
      ) : (
        <span className={cls}>{inner}</span>
      )}
      {error && <span className={s.fieldErrorText}>{error}</span>}
    </span>
  );
}
