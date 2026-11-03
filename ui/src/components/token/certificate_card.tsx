import { CertificateIcon } from '@phosphor-icons/react';

import type { Certificate } from '#types/bridge';
import { Card } from '#ui/card';
const dateFormat = new Intl.DateTimeFormat('en', {
  month: 'short',
  day: 'numeric',
  year: 'numeric',
});
export function CertificateCard({ certificate }: { certificate: Certificate }) {
  const expires = certificate.expiresUnix
    ? dateFormat.format(new Date(certificate.expiresUnix * 1000))
    : certificate.expires.split(' +')[0];
  return (
    <Card>
      <div className='certificate-symbol'>
        <CertificateIcon size={21} />
      </div>
      <div className='certificate-copy'>
        <strong title={certificate.subject}>{certificate.subject}</strong>
        <p>
          {certificate.issuer} · Expires {expires}
        </p>
      </div>
      <span className='token-tag'>{certificate.valid ? 'Certificate' : 'Unavailable'}</span>
    </Card>
  );
}
