import { LockKeyIcon, SignInIcon } from '@phosphor-icons/react';
import { useRef, useState } from 'react';
import type { SubmitEvent } from 'react';

import { AppMark } from '#components/layout/app_mark';
import type { ApproveLogin, PendingLogin, RejectLogin } from '#types/bridge';
import { Button } from '#ui/button';
import { Input } from '#ui/input';
interface LoginFormProps {
  pending: PendingLogin;
  busy: boolean;
  approve: ApproveLogin;
  reject: RejectLogin;
}
export function LoginForm({ pending, busy, approve, reject }: LoginFormProps) {
  const pin = useRef<HTMLInputElement>(null);
  const confirmation = useRef<HTMLInputElement>(null);
  const [message, setMessage] = useState('');
  const signing = busy || pending.signing;
  async function cancel() {
    if (signing) return;
    if (pin.current) pin.current.value = '';
    try {
      await reject(pending.id);
    } catch (error) {
      setMessage(String(error));
    }
  }
  async function submit(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    if (signing || !pin.current) return;
    let secret = pin.current.value;
    pin.current.value = '';
    setMessage('');
    try {
      const operation = approve(pending.id, secret, confirmation.current?.checked || false);
      secret = '';
      await operation;
    } catch (error) {
      setMessage(String(error));
    } finally {
      secret = '';
    }
  }
  return (
    <form onSubmit={submit} autoComplete='off'>
      <div className='dialog-symbol'>
        <AppMark large />
      </div>
      <span className='eyebrow'>LOGIN REQUEST</span>
      <h2 id='approval-title'>Approve your login</h2>
      <p id='approval-description'>MPass is asking to use your signing token.</p>
      <div className='request-details'>
        <div>
          <span>Website</span>
          <strong>{new URL(pending.origin).hostname}</strong>
        </div>
        <div>
          <span>Certificate</span>
          <strong>{pending.name}</strong>
        </div>
      </div>
      <label className='pin-label' htmlFor='pin'>
        Token PIN
      </label>
      <Input
        ref={pin}
        id='pin'
        type='password'
        placeholder='Enter your PIN'
        autoComplete='off'
        maxLength={128}
        required
        spellCheck={false}
        autoCapitalize='off'
        disabled={signing}
        data-autofocus
      />
      <label htmlFor='final_try' className='warning' hidden={!pending.pinFinalTry}>
        <Input id='final_try' ref={confirmation} type='checkbox' disabled={signing} /> One attempt
        remains. I understand that an incorrect PIN will lock my token.
      </label>
      {message && <output className='dialog-message'>{message}</output>}
      <div className='dialog-actions'>
        <Button variant='secondary' onClick={cancel} disabled={signing}>
          Cancel
        </Button>
        <Button type='submit' disabled={signing}>
          <SignInIcon size={15} />
          {signing ? 'Signing…' : 'Approve login'}
        </Button>
      </div>
      <p className='pin-footnote'>
        <LockKeyIcon size={12} /> Your PIN stays on this Mac and is not saved.
      </p>
      <details className='privacy'>
        <summary>What is shared?</summary>
        <p>
          MPass receives your certificate and login signature. Moldova’s timestamp service receives
          a signature digest and your certificate serial number.
        </p>
      </details>
    </form>
  );
}
