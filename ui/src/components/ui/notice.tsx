import type { NoticeMessage } from '#types/bridge';
export function Notice({ notice }: { notice: NoticeMessage | null }) {
  if (!notice) return null;
  return <output className={`notice${notice.error ? ' error' : ''}`}>{notice.message}</output>;
}
