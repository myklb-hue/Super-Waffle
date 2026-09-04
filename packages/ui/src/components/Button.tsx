import { Icon } from './Icon';
import type { IconName } from './icons';
import s from './ui.module.css';

export interface ButtonProps {
  label: string;
  icon?: IconName;
  variant?: 'primary' | 'default' | 'danger';
  disabled?: boolean;
  /** A spinner replaces the icon; the label stays, so the width does not jump. */
  loading?: boolean;
  onClick?: () => void;
}

export function Button({
  label,
  icon,
  variant = 'default',
  disabled = false,
  loading = false,
  onClick,
}: ButtonProps) {
  const cls = [
    s.button,
    variant === 'primary' && s.buttonPrimary,
    variant === 'danger' && s.buttonDanger,
  ]
    .filter(Boolean)
    .join(' ');
  return (
    <button
      type="button"
      className={cls}
      disabled={disabled || loading}
      aria-busy={loading || undefined}
      onClick={onClick}
    >
      {loading ? <span className={s.spinner} /> : icon ? <Icon name={icon} size={12} /> : null}
      {label}
    </button>
  );
}
