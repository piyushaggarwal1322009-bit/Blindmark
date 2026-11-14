import type { ActivityRecord, JournalRecord, NodeRecord, SecurityControl, TaskRecord } from '../types';

const now = Date.now();
const ago = (minutes: number) => new Date(now - minutes * 60_000).toISOString();
const demoDigest = (prefix: string, suffix: string) => `${prefix}${'0'.repeat(64 - prefix.length - suffix.length)}${suffix}`;

export const demoNodes: NodeRecord[] = [
  {
    id: 'node-leader-01', name: 'Leader North', role: 'Leader', status: 'Connected',
    heartbeat: ago(1), version: '0.6.2-demo', host: 'leader-01.blindmark.test', region: 'us-east-lab',
    detail: 'Primary coordination role. Connection state is a demonstration fixture, not a live health check.', demo: true,
  },
  {
    id: 'node-helper-01', name: 'Helper West', role: 'Helper', status: 'Connected',
    heartbeat: ago(2), version: '0.6.2-demo', host: 'helper-01.blindmark.test', region: 'us-west-lab',
    detail: 'Helper coordination role. Authenticated IPC and mTLS are not configured in this demonstration.', demo: true,
  },
  {
    id: 'node-helper-02', name: 'Helper Reserve', role: 'Helper', status: 'Unavailable',
    heartbeat: ago(42), version: '0.6.1-demo', host: 'helper-02.blindmark.test', region: 'eu-central-lab',
    detail: 'Example unavailable node used to demonstrate a degraded-state view.', demo: true,
  },
];

export const demoTasks: TaskRecord[] = [
  { id: 'task-8f31c2', name: 'Q4 compensation benchmark', node: 'Leader North', status: 'Running', progress: 68, createdAt: ago(1580), updatedAt: ago(3), category: 'Compensation', description: 'Demonstration campaign coordination task.', demo: true },
  { id: 'task-2d70a1', name: 'Regional response latency', node: 'Helper West', status: 'Pending', progress: null, createdAt: ago(3440), updatedAt: ago(18), category: 'Infrastructure', description: 'Demonstration task awaiting an operator review.', demo: true },
  { id: 'task-b51e94', name: 'Benefits enrollment pulse', node: 'Leader North', status: 'Completed', progress: 100, createdAt: ago(9980), updatedAt: ago(126), category: 'Workforce', description: 'Completed demonstration workflow; no backend processing occurred.', demo: true },
  { id: 'task-73c440', name: 'Helper failover rehearsal', node: 'Helper Reserve', status: 'Failed', progress: 41, createdAt: ago(12020), updatedAt: ago(241), category: 'Resilience', description: 'Example failure record for interface preview.', demo: true },
  { id: 'task-6a0d81', name: 'Privacy threshold review', node: 'Leader North', status: 'Queued', progress: 0, createdAt: ago(1340), updatedAt: ago(1340), category: 'Governance', description: 'Queued demonstration task.', demo: true },
  { id: 'task-c42f19', name: 'Legacy batch replay', node: 'Helper West', status: 'Cancelled', progress: null, createdAt: ago(16600), updatedAt: ago(980), category: 'Coordination', description: 'Cancelled demonstration task.', demo: true },
];

export const demoJournal: JournalRecord[] = [
  { id: 'msg-7c19a2', sender: 'leader-north', recipient: 'helper-west', taskId: 'task-8f31c2', sequence: 1842, protocolVersion: 'v1', receivedAt: ago(2), state: 'PROCESSED', digest: demoDigest('c83d9a74b203', 'e109'), hmacStatus: 'Unverified', senderAuthenticationStatus: 'Unavailable', processingDetail: 'Demo-only state fixture; no backend processing confirmation is connected.', demo: true },
  { id: 'msg-a8f520', sender: 'helper-west', recipient: 'leader-north', taskId: 'task-8f31c2', sequence: 904, protocolVersion: 'v1', receivedAt: ago(7), state: 'RECEIVED_PENDING', digest: demoDigest('823bd1c933fe', '7a01'), hmacStatus: 'Unverified', senderAuthenticationStatus: 'Unavailable', demo: true },
  { id: 'msg-10de44', sender: 'leader-north', recipient: 'helper-west', taskId: 'task-2d70a1', sequence: 1841, protocolVersion: 'v1', receivedAt: ago(19), state: 'REJECTED', digest: demoDigest('7ad0f639c0d0', '912a'), hmacStatus: 'Unverified', senderAuthenticationStatus: 'Unavailable', rejectionDetail: 'Synthetic rejection scenario; no live validation occurred.', demo: true },
  { id: 'msg-d35291', sender: 'leader-north', recipient: 'helper-west', taskId: 'task-b51e94', sequence: 1839, protocolVersion: 'v1', receivedAt: ago(48), state: 'PROCESSED', digest: demoDigest('ba510c6c72e1', '500f'), hmacStatus: 'Unverified', senderAuthenticationStatus: 'Unavailable', processingDetail: 'Demo-only state fixture; no backend processing confirmation is connected.', demo: true },
  { id: 'msg-4bc20f', sender: 'helper-west', recipient: 'leader-north', taskId: 'task-8f31c2', sequence: 903, protocolVersion: 'v1', receivedAt: ago(71), state: 'PROCESSED', digest: demoDigest('506ac4a20d53', 'ad71'), hmacStatus: 'Unverified', senderAuthenticationStatus: 'Unavailable', processingDetail: 'Demo-only state fixture; no backend processing confirmation is connected.', demo: true },
  { id: 'msg-e93a18', sender: 'leader-north', recipient: 'helper-west', taskId: 'task-b51e94', sequence: 1838, protocolVersion: 'v1', receivedAt: ago(132), state: 'PROCESSED', digest: demoDigest('0d94810e833e', '01b6'), hmacStatus: 'Unverified', senderAuthenticationStatus: 'Unavailable', processingDetail: 'Demo-only state fixture; no backend processing confirmation is connected.', demo: true },
  { id: 'msg-9f4d11', sender: 'leader-north', recipient: 'helper-west', taskId: 'task-6a0d81', sequence: 1843, protocolVersion: 'v1', receivedAt: ago(177), state: 'RECEIVED_PENDING', digest: demoDigest('623d8a090ef1', 'ba7d'), hmacStatus: 'Unverified', senderAuthenticationStatus: 'Unavailable', demo: true },
  { id: 'msg-553b70', sender: 'helper-west', recipient: 'leader-north', taskId: 'task-b51e94', sequence: 902, protocolVersion: 'v1', receivedAt: ago(263), state: 'PROCESSED', digest: demoDigest('aa184a10d020', 'dc11'), hmacStatus: 'Unverified', senderAuthenticationStatus: 'Unavailable', processingDetail: 'Demo-only state fixture; no backend processing confirmation is connected.', demo: true },
];

export const demoActivity: ActivityRecord[] = [
  { id: 'evt-01', event: 'Message processing completed', source: 'Leader North', category: 'Journal', timestamp: ago(2), status: 'Success', detail: 'Demo event. No live coordination message was processed.', demo: true },
  { id: 'evt-02', event: 'Inbound message pending', source: 'Helper West', category: 'Communication', timestamp: ago(7), status: 'Info', detail: 'Demo pending receipt requiring a future recovery policy.', demo: true },
  { id: 'evt-03', event: 'Conflicting message identity rejected', source: 'Leader North', category: 'Security', timestamp: ago(19), status: 'Warning', detail: 'Illustrative rejection event; not a verified security alert.', demo: true },
  { id: 'evt-04', event: 'Task marked complete', source: 'Leader North', category: 'Task', timestamp: ago(126), status: 'Success', detail: 'Demonstration task state only.', demo: true },
  { id: 'evt-05', event: 'Node heartbeat unavailable', source: 'Helper Reserve', category: 'Node', timestamp: ago(242), status: 'Warning', detail: 'Demonstration availability state only.', demo: true },
  { id: 'evt-06', event: 'Journal integrity check unavailable', source: 'System', category: 'System', timestamp: ago(322), status: 'Info', detail: 'No live journal integrity probe is connected to this UI.', demo: true },
  { id: 'evt-07', event: 'Task queued', source: 'Leader North', category: 'Task', timestamp: ago(510), status: 'Info', detail: 'Demonstration task state only.', demo: true },
  { id: 'evt-08', event: 'Service configuration changed', source: 'System', category: 'System', timestamp: ago(940), status: 'Info', detail: 'Synthetic audit event for layout preview.', demo: true },
];

export const demoSeries = [
  { time: '00:00', received: 28, processed: 26, rejected: 2 },
  { time: '02:00', received: 22, processed: 21, rejected: 1 },
  { time: '04:00', received: 35, processed: 32, rejected: 3 },
  { time: '06:00', received: 31, processed: 29, rejected: 2 },
  { time: '08:00', received: 47, processed: 44, rejected: 3 },
  { time: '10:00', received: 42, processed: 40, rejected: 2 },
  { time: '12:00', received: 58, processed: 53, rejected: 5 },
  { time: '14:00', received: 51, processed: 48, rejected: 3 },
  { time: '16:00', received: 66, processed: 61, rejected: 5 },
  { time: '18:00', received: 57, processed: 54, rejected: 3 },
  { time: '20:00', received: 73, processed: 68, rejected: 5 },
  { time: '22:00', received: 62, processed: 59, rejected: 3 },
];

export const securityControls: SecurityControl[] = [
  { id: 'hmac', title: 'HMAC frame verification', area: 'Message authentication', state: 'Implemented', detail: 'Canonical frame codec and verification exist in the isolated Rust crate. No key provisioning or route integration.', approved: true, demo: true },
  { id: 'journal-boundary', title: 'Journal transaction boundary', area: 'Durable processing', state: 'Verified', detail: 'The HIGH callback transaction escape was remediated and regression-tested.', approved: true, demo: true },
  { id: 'identity-unique', title: 'Accepted identity uniqueness', area: 'Replay protection', state: 'Planned', detail: 'Approved tuple; database-level uniqueness and separate conflict storage are not implemented.', approved: true, demo: true },
  { id: 'freshness', title: 'Freshness and clock policy', area: 'Replay protection', state: 'Planned', detail: '31.75-second lifetime and 2-second skew are approved design, not enforced in code.', approved: true, demo: true },
  { id: 'mTLS', title: 'Proxy mTLS', area: 'Transport security', state: 'Not Configured', detail: 'No proxy or mTLS connection is configured by this repository.', approved: true, demo: true },
  { id: 'local-ipc', title: 'Authenticated local IPC', area: 'Service isolation', state: 'Not Configured', detail: 'OS-authenticated IPC and access controls are approved design, not implemented.', approved: true, demo: true },
  { id: 'recovery-integrity', title: 'Backup and recovery integrity', area: 'Journal integrity', state: 'Unavailable', detail: 'Digest checks do not authenticate a coherently rewritten database; HMAC recovery verification is not claimed.', approved: true, demo: true },
  { id: 'parser-gate', title: 'Independent parser security gate', area: 'Route protection', state: 'Planned', detail: 'Parser improvements exist; independent review and complete framing tests remain required before routes.', approved: true, demo: true },
];
