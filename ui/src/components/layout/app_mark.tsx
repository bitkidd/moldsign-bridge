import { KeyIcon } from '@phosphor-icons/react';
export function AppMark({ large = false }: { large?: boolean }) {
  return (
    <div className={`app-mark${large ? ' large' : ''}`}>
      <KeyIcon weight='bold' />
    </div>
  );
}
