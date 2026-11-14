import type { JournalRecord, MessageStatus } from '../types';

export type JournalSortKey = 'sequence' | 'received' | 'state';
export type JournalSortDirection = 'asc' | 'desc';
export type JournalTimeRange = 'Any time' | 'Last hour' | 'Last 24 hours' | 'Last 7 days';

export interface JournalFilters {
  query: string;
  state: MessageStatus | 'All stored states';
  sender: string;
  recipient: string;
  timeRange: JournalTimeRange;
}

const rangeMinutes: Record<JournalTimeRange, number | null> = {
  'Any time': null,
  'Last hour': 60,
  'Last 24 hours': 1440,
  'Last 7 days': 10080,
};

export function filterJournalRecords(records: readonly JournalRecord[], filters: JournalFilters, now = Date.now()): JournalRecord[] {
  const normalizedQuery = filters.query.trim().toLocaleLowerCase();
  const minutes = rangeMinutes[filters.timeRange];
  const cutoff = minutes === null ? null : now - minutes * 60_000;

  return records.filter((record) => {
    if (filters.state !== 'All stored states' && record.state !== filters.state) return false;
    if (filters.sender !== 'All senders' && record.sender !== filters.sender) return false;
    if (filters.recipient !== 'All recipients' && record.recipient !== filters.recipient) return false;
    if (cutoff !== null) {
      const receivedAt = Date.parse(record.receivedAt);
      if (receivedAt < cutoff || receivedAt > now) return false;
    }
    if (!normalizedQuery) return true;

    return [record.id, record.taskId, record.sender, record.recipient, record.digest]
      .some((value) => value.toLocaleLowerCase().includes(normalizedQuery));
  });
}

export function sortJournalRecords(
  records: readonly JournalRecord[],
  sortKey: JournalSortKey,
  direction: JournalSortDirection,
): JournalRecord[] {
  const multiplier = direction === 'asc' ? 1 : -1;
  return [...records].sort((left, right) => {
    const comparison = sortKey === 'sequence'
      ? left.sequence - right.sequence
      : sortKey === 'received'
        ? Date.parse(left.receivedAt) - Date.parse(right.receivedAt)
        : left.state.localeCompare(right.state);
    return comparison * multiplier;
  });
}
