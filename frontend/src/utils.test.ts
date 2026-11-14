import { describe, expect, it } from 'vitest';
import { filterByQuery } from './utils';
import { filterJournalRecords, sortJournalRecords } from './pages/journal-utils';
import { demoJournal } from './data/demo';
import type { JournalRecord } from './types';

const records = [
  { id: 'a1', state: 'PROCESSED' },
  { id: 'b2', state: 'RECEIVED_PENDING' },
  { id: 'c3', state: 'REJECTED' },
];

const journalRecords: JournalRecord[] = [
  { id: 'msg-3', sender: 'leader', recipient: 'helper', taskId: 'task-3', sequence: 3, protocolVersion: 'v1', receivedAt: '2026-11-14T12:10:00Z', state: 'PROCESSED', digest: 'c'.repeat(64), hmacStatus: 'Unverified', senderAuthenticationStatus: 'Unavailable', demo: true },
  { id: 'msg-1', sender: 'helper', recipient: 'leader', taskId: 'task-1', sequence: 1, protocolVersion: 'v1', receivedAt: '2026-11-14T11:10:00Z', state: 'RECEIVED_PENDING', digest: 'a'.repeat(64), hmacStatus: 'Unverified', senderAuthenticationStatus: 'Unavailable', demo: true },
  { id: 'msg-2', sender: 'leader', recipient: 'helper', taskId: 'task-2', sequence: 2, protocolVersion: 'v1', receivedAt: '2026-11-14T10:10:00Z', state: 'REJECTED', digest: 'b'.repeat(64), hmacStatus: 'Unverified', senderAuthenticationStatus: 'Unavailable', demo: true },
];

describe('filterByQuery', () => {
  it('returns all rows for an empty query', () => {
    expect(filterByQuery(records, '  ', ['id', 'state'])).toEqual(records);
  });

  it('matches selected fields without case sensitivity', () => {
    expect(filterByQuery(records, 'pending', ['id', 'state'])).toEqual([records[1]]);
  });
});

describe('journal filters and sorting', () => {
  it('combines query, state, sender, recipient, and received-time filters', () => {
    expect(filterJournalRecords(journalRecords, {
      query: 'TASK-2',
      state: 'REJECTED',
      sender: 'leader',
      recipient: 'helper',
      timeRange: 'Last hour',
    }, Date.parse('2026-11-14T10:20:00Z')).map((record) => record.id)).toEqual(['msg-2']);
  });

  it('sorts sequence numbers numerically in both directions', () => {
    expect(sortJournalRecords(journalRecords, 'sequence', 'asc').map((record) => record.sequence)).toEqual([1, 2, 3]);
    expect(sortJournalRecords(journalRecords, 'sequence', 'desc').map((record) => record.sequence)).toEqual([3, 2, 1]);
  });

  it('sorts received timestamps and journal states', () => {
    expect(sortJournalRecords(journalRecords, 'received', 'desc').map((record) => record.id)).toEqual(['msg-3', 'msg-1', 'msg-2']);
    expect(sortJournalRecords(journalRecords, 'state', 'asc').map((record) => record.state)).toEqual(['PROCESSED', 'RECEIVED_PENDING', 'REJECTED']);
  });
});

describe('journal demonstration fixtures', () => {
  it('uses full digest-shaped values without claiming verification', () => {
    expect(demoJournal.every((record) => /^[0-9a-f]{64}$/.test(record.digest))).toBe(true);
    expect(demoJournal.every((record) => record.hmacStatus === 'Unverified')).toBe(true);
    expect(demoJournal.every((record) => record.senderAuthenticationStatus === 'Unavailable')).toBe(true);
  });
});
