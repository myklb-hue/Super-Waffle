import type { CSSProperties } from 'react';
import { ICONS, type IconName } from './icons';
import { colorVar, type ColorToken } from '../types';
import s from './ui.module.css';

/** The sizes the artboards actually use. */
export type IconSize = 10 | 11 | 12 | 13 | 14 | 18;

export interface IconProps {
  name: IconName;
  size?: IconSize;
  color?: ColorToken;
  strokeWidth?: number;
  style?: CSSProperties;
}

/** A glyph from the shared set. The markup comes from design/cyberloom, so an
 *  icon here is the icon the mockups draw. */
export function Icon({ name, size = 14, color, strokeWidth = 1.6, style }: IconProps) {
  return (
    <svg
      className={s.icon}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      // A glyph drawn as a solid shape asks for no stroke, and until now that
      // made it invisible: the fill was hardcoded to none, so `play` — the
      // Run button's own triangle — rendered as nothing at all. A stroke width
      // of zero means filled.
      fill={strokeWidth === 0 ? colorVar(color) : 'none'}
      stroke={strokeWidth === 0 ? 'none' : colorVar(color)}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      style={style}
      dangerouslySetInnerHTML={{ __html: ICONS[name] }}
    />
  );
}
