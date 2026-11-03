export interface Inspection {
  driver: boolean;
  driverPath: string | null;
  architecture: string;
}
export interface PendingLogin {
  id: string;
  origin: string;
  name: string;
  signing: boolean;
  pinFinalTry: boolean;
}
export interface ServiceStatus {
  running: boolean;
  trusted: boolean;
  certificatePath: string;
  message: string;
  pending: PendingLogin | null;
}
export interface Certificate {
  subject: string;
  issuer: string;
  expires: string;
  expiresUnix?: number;
  fingerprint: string;
  valid: boolean;
}
export interface Token {
  label: string;
  model: string;
  pinLocked: boolean;
  pinFinalTry: boolean;
  certificates: Certificate[];
}
export type ProbeResult =
  | { status: 'connected' | 'no_token'; tokens: Token[] }
  | { status: 'error'; message: string };
export interface NoticeMessage {
  message: string;
  error: boolean;
}
export type ApproveLogin = (id: string, pin: string, finalTryConfirmed: boolean) => Promise<void>;
export type RejectLogin = (id: string) => Promise<void>;
