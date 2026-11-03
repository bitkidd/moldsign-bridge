import { CaretRightIcon } from '@phosphor-icons/react';

import { AppUpdates } from '#components/connection/app_updates';
import type { ServiceStatus } from '#types/bridge';
export function ConnectionDetails({
  ready,
  driver,
  service,
  busy,
}: {
  busy: boolean;
  ready: boolean;
  driver: boolean;
  service: ServiceStatus | null;
}) {
  return (
    <details className='connection-details'>
      <summary>
        <CaretRightIcon size={12} />
        Connection details <span>{ready ? 'All checks passed' : 'Needs attention'}</span>
      </summary>
      <dl>
        <div>
          <dt>Secure connection</dt>
          <dd>{service?.trusted ? 'Verified on this Mac' : 'Setup required'}</dd>
        </div>
        <div>
          <dt>SafeNet driver</dt>
          <dd>{driver ? 'Installed' : 'Not found'}</dd>
        </div>
        <div>
          <dt>Login service</dt>
          <dd>{service?.running ? 'Running' : 'Stopped'}</dd>
        </div>
      </dl>
      <AppUpdates disabled={busy || Boolean(service?.pending)} />
      <p>Setup is remembered on this Mac. Your PIN is never saved.</p>
    </details>
  );
}
