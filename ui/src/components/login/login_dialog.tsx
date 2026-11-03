import { useState } from 'react';

import { LoginForm } from '#forms/login_form';
import type { ApproveLogin, PendingLogin, RejectLogin } from '#types/bridge';
import { Dialog } from '#ui/dialog';
interface LoginDialogProps {
  pending: PendingLogin | null;
  busy: boolean;
  approve: ApproveLogin;
  reject: RejectLogin;
}
export function LoginDialog({ pending, busy, approve, reject }: LoginDialogProps) {
  const [cancelError, setCancelError] = useState('');
  return (
    <Dialog
      id='approval'
      open={Boolean(pending)}
      aria-labelledby='approval-title'
      aria-describedby='approval-description'
      onCancel={() => {
        if (pending && !busy && !pending.signing)
          void reject(pending.id).catch((error: unknown) => setCancelError(String(error)));
      }}
    >
      {cancelError && <output className='dialog-message'>{cancelError}</output>}
      {pending && (
        <LoginForm
          key={pending.id}
          pending={pending}
          busy={busy}
          approve={approve}
          reject={reject}
        />
      )}
    </Dialog>
  );
}
