import { request } from './client';

export interface TlsEndpointStatus {
  tlsEnabled: boolean;
  mtlsRequired: boolean;
  certConfigured: boolean;
  keyConfigured: boolean;
  caConfigured: boolean;
  listenerMode: string;
}

export interface TransportSecurityStatusResponse {
  http: TlsEndpointStatus;
  workerTunnel: TlsEndpointStatus;
  ready: boolean;
  issues: string[];
}

export interface SecurityPostureCheck {
  id: string;
  label: string;
  status: 'ok' | 'warning' | 'critical' | string;
  source: string;
  detail: string;
  evidenceCount: number;
}

export interface ScriptGovernancePosture {
  totalScripts: number;
  safeDefaultDenyScripts: number;
  dangerousPolicyScripts: number;
  releasedScripts: number;
  signedReleases: number;
  releasesWithGrants: number;
  releaseSignatureRequired: boolean;
  releaseSignatureSecretConfigured: boolean;
}

export interface NotificationSafetyPosture {
  totalChannels: number;
  enabledChannels: number;
  configuredTargets: number;
  redactedTargets: number;
  channelsWithSafetyPolicy: number;
  directSecretValuesRedacted: number;
}

export interface ClusterTransportPosture {
  raftTransportTokenConfigured: boolean;
  workerTunnelTlsReady: boolean;
  httpTlsReady: boolean;
}

export interface SecurityPolicyDenial {
  id: string;
  resourceType: string;
  resourceId: string;
  action: string;
  failureReason: string;
  detail: string | null;
  createdAt: string;
}

export interface SecurityPostureResponse {
  overallStatus: 'ok' | 'warning' | 'critical' | string;
  checks: SecurityPostureCheck[];
  transport: TransportSecurityStatusResponse;
  scriptGovernance: ScriptGovernancePosture;
  notificationSafety: NotificationSafetyPosture;
  clusterTransport: ClusterTransportPosture;
  recentDenials: SecurityPolicyDenial[];
}

export function getSecurityPosture(): Promise<SecurityPostureResponse> {
  return request<SecurityPostureResponse>('/api/v1/security/posture');
}
