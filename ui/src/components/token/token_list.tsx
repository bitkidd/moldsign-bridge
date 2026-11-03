import { ArrowClockwiseIcon } from '@phosphor-icons/react';

import { CertificateCard } from '#components/token/certificate_card';
import type { Token } from '#types/bridge';
import { Button } from '#ui/button';
interface TokenListProps {
  tokens: Token[];
  message: string;
  scanning: boolean;
  disabled: boolean;
  onRefresh: () => Promise<void>;
}
export function TokenList({ tokens, message, scanning, disabled, onRefresh }: TokenListProps) {
  const certificates = tokens.flatMap((token) => token.certificates);
  return (
    <section className='certificates' aria-labelledby='certificates-title'>
      <div className='section-heading'>
        <h3 id='certificates-title'>Your signing token</h3>
        <Button
          variant='icon'
          onClick={onRefresh}
          disabled={scanning || disabled}
          className={scanning ? 'loading' : ''}
          aria-label='Refresh token'
          title='Refresh token'
        >
          <ArrowClockwiseIcon size={16} />
        </Button>
      </div>
      <div aria-live='polite'>
        {certificates.length ? (
          certificates.map((certificate) => (
            <CertificateCard key={certificate.fingerprint} certificate={certificate} />
          ))
        ) : (
          <div className='empty-state'>{message}</div>
        )}
      </div>
    </section>
  );
}
