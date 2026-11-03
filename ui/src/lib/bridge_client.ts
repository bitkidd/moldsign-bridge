import { invoke } from '@tauri-apps/api/core';

import type { Inspection, ProbeResult, ServiceStatus } from '#types/bridge';

export const bridgeClient = {
  inspect: () => invoke<Inspection>('inspect'),
  status: () => invoke<ServiceStatus>('service_status'),
  probe: () => invoke<ProbeResult>('probe'),
  start: () => invoke<void>('start_service'),
  trust: () => invoke<void>('trust_connection'),
  openMpass: () => invoke<void>('open_mpass'),
  reject: (id: string) => invoke<void>('reject_login', { id }),
  approve: (id: string, pin: string, finalTryConfirmed: boolean) =>
    invoke<void>('approve_login', { id, pin, finalTryConfirmed }),
};
