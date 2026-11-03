import { ArrowUpRightIcon, ShieldCheckIcon } from '@phosphor-icons/react';

import type { ServiceStatus } from '#types/bridge';
import { Button } from '#ui/button';
function connectionCopy(driver: boolean, service: ServiceStatus | null) {
  if (!service)
    return {
      title: 'Getting things ready',
      copy: 'Checking your secure connection.',
      action: 'Connecting…',
      badge: 'Connecting',
    };
  if (!driver)
    return {
      title: 'SafeNet is missing',
      copy: 'Install SafeNet Authentication Client to connect your signing token.',
      action: 'Check again',
      badge: 'Driver needed',
    };
  if (!service.running)
    return {
      title: 'Connection is paused',
      copy: 'Reconnect to make your token available to MPass.',
      action: 'Reconnect',
      badge: 'Offline',
    };
  if (!service.trusted)
    return {
      title: 'One step to get started',
      copy: 'Allow a secure connection between MPass and this app.',
      action: 'Set up connection',
      badge: 'Setup needed',
    };
  return {
    title: 'Ready when you are',
    copy: 'Open MPass and choose your signing certificate. We’ll take it from there.',
    action: 'Open MPass',
    badge: 'Connected',
  };
}
interface ConnectionStatusProps {
  driver: boolean;
  service: ServiceStatus | null;
  busy: boolean;
  onAction: () => Promise<void>;
}
export function ConnectionStatus({ driver, service, busy, onAction }: ConnectionStatusProps) {
  const ready = Boolean(driver && service?.running && service.trusted);
  const copy = connectionCopy(driver, service);
  return (
    <section className='connection' aria-labelledby='connection-title'>
      <div className='connection-art'>
        <ShieldCheckIcon weight='light' />
      </div>
      <span className={`status-badge${service && !ready ? ' attention' : ''}`}>{copy.badge}</span>
      <h2 id='connection-title'>{copy.title}</h2>
      <p>{copy.copy}</p>
      <Button onClick={onAction} disabled={busy || !service}>
        {busy ? 'Please wait…' : copy.action}
        {ready && !busy && <ArrowUpRightIcon size={14} />}
      </Button>
      {service?.running && !service.trusted && (
        <p className='setup-note'>
          One-time setup. macOS will ask to trust this app’s local connection.
        </p>
      )}
    </section>
  );
}
