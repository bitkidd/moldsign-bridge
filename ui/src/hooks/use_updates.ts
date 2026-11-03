import { invoke } from '@tauri-apps/api/core';
import { useState } from 'react';

export function useUpdates() {
  const [busy, setBusy] = useState(false);
  const [version, setVersion] = useState<string | null>(null);
  const [message, setMessage] = useState('Updates are installed only when you choose.');
  async function check() {
    setBusy(true);
    setVersion(null);
    setMessage('Checking for updates…');
    try {
      const next = await invoke<string | null>('check_update');
      setVersion(next);
      setMessage(next ? `Alpha ${next} is available.` : 'You’re up to date.');
    } catch {
      setMessage('Could not reach the Alpha release channel. Please try again later.');
    } finally {
      setBusy(false);
    }
  }
  async function install() {
    setBusy(true);
    setMessage('Downloading and verifying the update… The app will restart.');
    try {
      await invoke('install_update');
    } catch {
      setMessage('Update could not be installed. Finish any login and try again.');
      setBusy(false);
    }
  }
  return { busy, version, message, check, install };
}
