import type { ComponentProps } from 'react';
type ButtonProps = ComponentProps<'button'> & { variant?: 'primary' | 'secondary' | 'icon' };
export function Button({
  variant = 'primary',
  className = '',
  type = 'button',
  ...props
}: ButtonProps) {
  return (
    <button
      type={type}
      className={`${variant === 'icon' ? 'icon-button' : variant} ${className}`.trim()}
      {...props}
    />
  );
}
