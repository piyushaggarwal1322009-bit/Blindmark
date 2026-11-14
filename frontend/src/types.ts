export type NodeStatus = 'Connected' | 'Disconnected' | 'Connecting' | 'Unavailable' | 'Authentication Required' | 'Error';
export type TaskStatus = 'Queued' | 'Pending' | 'Running' | 'Completed' | 'Failed' | 'Cancelled';
export type MessageStatus = 'RECEIVED_PENDING' | 'PROCESSED' | 'REJECTED';
export type VerificationStatus = 'Unavailable' | 'Unverified' | 'Verified';
export type SecurityState = 'Implemented' | 'Verified' | 'Planned' | 'Not Configured' | 'Unavailable';

export interface NodeRecord {
  id: string;
  name: string;
  role: 'Leader' | 'Helper';
  status: NodeStatus;
  heartbeat: string;
  version: string;
  host: string;
  region: string;
  detail: string;
  demo: true;
}

export interface TaskRecord {
  id: string;
  name: string;
  node: string;
  status: TaskStatus;
  progress: number | null;
  createdAt: string;
  updatedAt: string;
  category: string;
  description: string;
  demo: true;
}

export interface JournalRecord {
  id: string;
  sender: string;
  recipient: string;
  taskId: string;
  sequence: number;
  protocolVersion: string;
  receivedAt: string;
  state: MessageStatus;
  digest: string;
  hmacStatus: VerificationStatus;
  senderAuthenticationStatus: VerificationStatus;
  processingDetail?: string;
  rejectionDetail?: string;
  demo: true;
}

export interface ActivityRecord {
  id: string;
  event: string;
  source: string;
  category: 'Task' | 'Communication' | 'Node' | 'Security' | 'Journal' | 'System';
  timestamp: string;
  status: 'Success' | 'Warning' | 'Failed' | 'Info';
  detail: string;
  demo: true;
}

export interface SecurityControl {
  id: string;
  title: string;
  area: string;
  state: SecurityState;
  detail: string;
  approved: boolean;
  demo: true;
}

export interface SystemHealthResponse {
  status: 'ok';
  service: 'blindmark-backend';
}

export type ApiFailureKind = 'not-configured' | 'timeout' | 'network' | 'unauthorized' | 'unexpected';

export interface ApiFailure {
  kind: ApiFailureKind;
  message: string;
}
