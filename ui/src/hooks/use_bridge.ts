import { useCallback, useEffect, useRef, useState } from 'react';

import { bridgeClient } from '#lib/bridge_client';
import type { NoticeMessage, ServiceStatus, Token } from '#types/bridge';

export function useBridge() {
  const [driver, setDriver] = useState(false);
  const [service, setService] = useState<ServiceStatus | null>(null);
  const [tokens, setTokens] = useState<Token[]>([]);
  const [tokenMessage, setTokenMessage] = useState('Looking for your SafeNet token…');
  const [notice, setNotice] = useState<NoticeMessage | null>(null);
  const [busy, setBusy] = useState(false);
  const [scanning, setScanning] = useState(false);
  const refs = useRef<{
    busy: boolean;
    polling: boolean;
    scanning: boolean;
    service: ServiceStatus | null;
    message: string | null;
  }>({ busy: false, polling: false, scanning: false, service: null, message: null });
  const notify = useCallback(
    (message = '', error = false) => setNotice(message ? { message, error } : null),
    [],
  );

  const refresh = useCallback(async () => {
    const current = refs.current;
    if (current.busy || current.polling) return;
    current.polling = true;
    try {
      const next = await bridgeClient.status();
      current.service = next;
      setService(next);
      if (
        next.message !== current.message &&
        /Login (signature|failed|cancelled)|expired/i.test(next.message)
      ) {
        notify(next.message, /failed|expired/i.test(next.message));
      }
      current.message = next.message;
    } catch (error) {
      notify(String(error), true);
    } finally {
      current.polling = false;
    }
  }, [notify]);

  const scan = useCallback(async () => {
    const current = refs.current;
    if (current.scanning || current.busy || current.service?.pending) return;
    current.scanning = true;
    setScanning(true);
    try {
      const result = await bridgeClient.probe();
      if (result.status === 'error') throw new Error(result.message);
      setTokens(result.tokens || []);
      setTokenMessage(
        result.tokens?.length
          ? 'Token connected. No public certificates are available.'
          : 'Connect your SafeNet USB token, then refresh.',
      );
      for (const token of result.tokens || []) {
        if (token.pinLocked) notify('Your token PIN is locked. Contact your token provider.', true);
        else if (token.pinFinalTry) notify('Your token has one PIN attempt remaining.', true);
      }
    } catch (error) {
      setTokens([]);
      setTokenMessage(String(error).replace(/^Error: /, ''));
    } finally {
      current.scanning = false;
      setScanning(false);
    }
  }, [notify]);

  const perform = useCallback(
    async (operation: () => Promise<void>) => {
      if (refs.current.busy) throw new Error('An operation is already in progress.');
      refs.current.busy = true;
      setBusy(true);
      try {
        return await operation();
      } finally {
        refs.current.busy = false;
        setBusy(false);
        await refresh();
      }
    },
    [refresh],
  );

  const primaryAction = useCallback(async () => {
    notify();
    try {
      await perform(async () => {
        if (!driver) {
          const result = await bridgeClient.inspect();
          setDriver(result.driver);
        } else if (!refs.current.service?.running) await bridgeClient.start();
        else if (!refs.current.service.trusted) await bridgeClient.trust();
        else await bridgeClient.openMpass();
      });
    } catch (error) {
      notify(String(error), true);
    }
  }, [driver, notify, perform]);

  const reject = useCallback(
    async (id: string) => {
      if (refs.current.busy) return;
      await bridgeClient.reject(id);
      await refresh();
    },
    [refresh],
  );

  const approve = useCallback(
    async (id: string, pin: string, finalTryConfirmed: boolean) => {
      try {
        await perform(() => bridgeClient.approve(id, pin, finalTryConfirmed));
        notify('Signature created. MPass will verify your login.');
      } catch (error) {
        notify(String(error), true);
        throw error;
      } finally {
        pin = '';
      }
    },
    [perform, notify],
  );

  useEffect(() => {
    let disposed = false;
    async function initialize() {
      let installed = false;
      try {
        const result = await bridgeClient.inspect();
        if (disposed) return;
        installed = result.driver;
        setDriver(installed);
        await bridgeClient.start();
      } catch (error) {
        if (!disposed) notify(String(error), true);
      }
      if (disposed) return;
      await refresh();
      if (installed) await scan();
      else setTokenMessage('Install SafeNet Authentication Client to access your token.');
    }
    initialize();
    const interval = setInterval(refresh, 900);
    return () => {
      disposed = true;
      clearInterval(interval);
    };
  }, [notify, refresh, scan]);

  return {
    driver,
    service,
    tokens,
    tokenMessage,
    notice,
    busy,
    scanning,
    scan,
    primaryAction,
    approve,
    reject,
  };
}

export type BridgeState = ReturnType<typeof useBridge>;
