import type { ButtonHTMLAttributes } from 'react';

type Variant = 'default' | 'accent' | 'ghost' | 'good' | 'danger';

interface Props extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  small?: boolean;
}

/** The one button. `accent` is the filled primary; the rest are outline/ghost. */
export function Button({ variant = 'default', small, className, ...rest }: Props) {
  const cls = ['btn', `btn-${variant}`, small ? 'btn-sm' : '', className ?? '']
    .filter(Boolean)
    .join(' ');
  return <button className={cls} {...rest} />;
}
