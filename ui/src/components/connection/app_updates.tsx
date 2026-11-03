import { ArrowClockwiseIcon, DownloadSimpleIcon } from '@phosphor-icons/react';

import { useUpdates } from '#hooks/use_updates';
import { Button } from '#ui/button';

export function AppUpdates({ disabled }: { disabled: boolean }) {
  const updates = useUpdates();
  return (
    <section className='app-updates' aria-label='App updates'>
      <Button
        variant='secondary'
        disabled={disabled || updates.busy}
        onClick={updates.version ? updates.install : updates.check}
      >
        {updates.version ? <DownloadSimpleIcon size={16} /> : <ArrowClockwiseIcon size={16} />}
        {updates.version ? 'Install & restart' : 'Check for updates'}
      </Button>
      <output>{updates.message}</output>
    </section>
  );
}
