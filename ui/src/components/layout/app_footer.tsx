export function AppFooter({ ready }: { ready: boolean }) {
  return (
    <footer>
      <span className={`footer-dot${ready ? ' ready' : ''}`} />
      <span>{ready ? 'Listening for login requests' : 'Connection needs attention'}</span>
      <span>MPass · SafeNet</span>
    </footer>
  );
}
