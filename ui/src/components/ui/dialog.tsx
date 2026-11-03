import { useEffect, useRef } from 'react';
import type { ComponentProps } from 'react';
type DialogProps = Omit<ComponentProps<'dialog'>, 'open' | 'onCancel'> & {
  open: boolean;
  onCancel: () => void;
};
export function Dialog({ open, onCancel, children, ...props }: DialogProps) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const node = dialog.current;
    if (!node) return;
    if (open) {
      if (!node.open) node.showModal();
      node.querySelector<HTMLElement>('[data-autofocus]')?.focus();
    } else if (node.open) node.close();
  }, [open]);
  return (
    <dialog
      ref={dialog}
      onCancel={(event) => {
        event.preventDefault();
        onCancel();
      }}
      {...props}
    >
      {children}
    </dialog>
  );
}
