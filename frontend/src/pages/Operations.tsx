import { useEffect, useMemo, useState, type FormEvent } from 'react';
import { Activity, ArrowDown, ArrowUp, ArrowUpDown, Check, ChevronDown, ChevronRight, Clock3, Copy, Database, Download, Filter, KeyRound, MoreHorizontal, Network, Plus, RefreshCw, Shield, ShieldAlert, ShieldCheck, SlidersHorizontal, X } from 'lucide-react';
import { blindMarkApi, ApiRequestError } from '../services/api';
import { demoActivity, demoJournal, demoNodes, demoTasks, securityControls } from '../data/demo';
import type { ActivityRecord, JournalRecord, MessageStatus, NodeRecord, SecurityControl, TaskRecord } from '../types';
import { copyTextToClipboard, filterByQuery, formatDateTime, formatRelativeTime } from '../utils';
import { filterJournalRecords, sortJournalRecords, type JournalSortDirection, type JournalSortKey, type JournalTimeRange } from './journal-utils';
import { Badge, Button, DetailRow, Drawer, EmptyState, IconButton, Modal, PageHeader, ProgressBar, SearchInput, SectionTitle, StateBadge } from '../components/ui';
import { useToast } from '../hooks/useToast';

const selectOptions = (items: string[]) => items.map((item) => <option key={item} value={item}>{item}</option>);

export function TasksPage() {
  const [tasks, setTasks] = useState<TaskRecord[]>(demoTasks);
  const [query, setQuery] = useState('');
  const [statusFilter, setStatusFilter] = useState('All statuses');
  const [sortOrder, setSortOrder] = useState('updated');
  const [selected, setSelected] = useState<TaskRecord | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const notify = useToast();
  const filtered = useMemo(() => {
    const matching = filterByQuery(tasks, query, ['name', 'id', 'node', 'category']);
    const selectedRows = statusFilter === 'All statuses' ? matching : matching.filter((task) => task.status === statusFilter);
    return [...selectedRows].sort((a, b) => sortOrder === 'name' ? a.name.localeCompare(b.name) : Date.parse(b.updatedAt) - Date.parse(a.updatedAt));
  }, [tasks, query, statusFilter, sortOrder]);

  function createTask(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    const name = String(form.get('name') ?? '').trim();
    if (!name) return;
    const node = String(form.get('node') ?? demoNodes[0].name);
    const newTask: TaskRecord = {
      id: `task-${Math.random().toString(16).slice(2, 8)}`,
      name,
      node,
      status: 'Queued',
      progress: 0,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
      category: String(form.get('category') ?? 'Demonstration'),
      description: String(form.get('description') ?? 'Created locally in demonstration mode.'),
      demo: true,
    };
    setTasks((current) => [newTask, ...current]);
    setCreateOpen(false);
    notify({ title: 'Demo task created locally', detail: 'No task was submitted to a backend.', tone: 'success' });
  }

  return (
    <div className="page-stack">
      <PageHeader eyebrow="Workspace / 02" title="Tasks" description="Manage and monitor distributed workloads." actions={<><Badge tone="accent">DEMO DATA</Badge><Button variant="primary" onClick={() => setCreateOpen(true)}><Plus size={16} />New task</Button></>} />
      <div className="page-context-strip"><span className="context-icon"><WorkflowIcon /></span><span>6 demonstration tasks</span><span className="context-separator" /><span>Task creation is local to this preview</span></div>
      <section className="surface table-surface">
        <div className="table-toolbar"><div className="toolbar-search"><SearchInput value={query} onChange={setQuery} placeholder="Search name, ID, or node" label="Search tasks" /></div><label className="select-control"><span className="sr-only">Filter by status</span><Filter size={14} /><select value={statusFilter} onChange={(event) => setStatusFilter(event.target.value)}>{selectOptions(['All statuses', 'Queued', 'Pending', 'Running', 'Completed', 'Failed', 'Cancelled'])}</select><ChevronDown size={13} /></label><label className="select-control sort-control"><span className="sr-only">Sort tasks</span><SlidersHorizontal size={14} /><select value={sortOrder} onChange={(event) => setSortOrder(event.target.value)}><option value="updated">Recently updated</option><option value="name">Name A–Z</option></select><ChevronDown size={13} /></label><Button onClick={() => notify({ title: 'Export unavailable', detail: 'Task export is not connected to a backend.', tone: 'info' })}><Download size={15} />Export</Button></div>
        <div className="table-wrap"><table className="data-table"><thead><tr><th>Task</th><th>Assigned node</th><th>Status</th><th>Progress</th><th>Created</th><th>Last updated</th><th><span className="sr-only">Actions</span></th></tr></thead><tbody>
          {filtered.map((task) => <tr key={task.id} className="clickable-row" onClick={() => setSelected(task)}><td><button className="table-primary mono" onClick={(event) => { event.stopPropagation(); setSelected(task); }}>{task.name}<small>{task.id}</small></button></td><td><span className="node-cell"><span className="table-node-dot" />{task.node}</span></td><td><StateBadge value={task.status} /></td><td>{task.progress === null ? <span className="muted">—</span> : <div className="table-progress"><ProgressBar value={task.progress} /><span>{task.progress}%</span></div>}</td><td>{formatDateTime(task.createdAt)}</td><td>{formatRelativeTime(task.updatedAt)}</td><td><IconButton icon={MoreHorizontal} label={`More actions for ${task.name}`} onClick={(event) => { event.stopPropagation(); setSelected(task); }} /></td></tr>)}
          {filtered.length === 0 && <tr><td colSpan={7}><EmptyState title="No tasks found" detail="Try another search or status filter." /></td></tr>}
        </tbody></table></div>
        <div className="table-footer"><span>Showing <strong>{filtered.length}</strong> of {tasks.length} demo tasks</span><span>Source: local fixture</span></div>
      </section>
      <Modal open={createOpen} title="Create demonstration task" description="This form creates a local preview item only. It does not submit work to BlindMark." onClose={() => setCreateOpen(false)}>
        <form className="form-stack" onSubmit={createTask}>
          <label className="form-field"><span>Task name <b>*</b></span><input name="name" required maxLength={90} placeholder="e.g. Q4 response check" autoFocus data-autofocus="true" /></label>
          <label className="form-field"><span>Description</span><textarea name="description" rows={3} maxLength={240} placeholder="Short demonstration description" /></label>
          <div className="form-grid"><label className="form-field"><span>Assigned node</span><select name="node">{demoNodes.map((node) => <option key={node.id}>{node.name}</option>)}</select></label><label className="form-field"><span>Category</span><select name="category">{selectOptions(['Demonstration', 'Coordination', 'Resilience', 'Governance'])}</select></label></div>
          <div className="form-notice"><Shield size={15} /><span>Task APIs are not implemented. This item is held in page memory only.</span></div>
          <div className="dialog-actions"><Button type="button" onClick={() => setCreateOpen(false)}>Cancel</Button><Button type="submit" variant="primary"><Plus size={15} />Create demo task</Button></div>
        </form>
      </Modal>
      <Drawer open={selected !== null} onClose={() => setSelected(null)} title={selected?.name ?? 'Task details'} eyebrow="DEMONSTRATION TASK">
        {selected && <div className="detail-stack"><div className="detail-status"><span className="node-glyph task-glyph"><WorkflowIcon /></span><div><strong>{selected.category}</strong><small>Local fixture · not a live task</small></div><StateBadge value={selected.status} /></div><div className="detail-group"><DetailRow label="Task ID" value={selected.id} mono /><DetailRow label="Assigned node" value={selected.node} /><DetailRow label="Created" value={formatDateTime(selected.createdAt)} /><DetailRow label="Updated" value={formatDateTime(selected.updatedAt)} /><DetailRow label="Description" value={selected.description} /></div>{selected.progress !== null && <div className="detail-progress"><span>Demonstration progress <b>{selected.progress}%</b></span><ProgressBar value={selected.progress} /></div>}<div className="detail-callout"><Shield size={16} /><p>Details are demonstration data. This task has not been sent to a backend.</p></div></div>}
      </Drawer>
    </div>
  );
}

function WorkflowIcon() { return <Activity size={15} />; }

export function NodesPage() {
  const [query, setQuery] = useState('');
  const [role, setRole] = useState('All roles');
  const [selected, setSelected] = useState<NodeRecord | null>(null);
  const nodes = demoNodes.filter((node) => (role === 'All roles' || node.role === role) && filterByQuery([node], query, ['name', 'id', 'role', 'region'])[0]);
  return (
    <div className="page-stack">
      <PageHeader eyebrow="Workspace / 03" title="Nodes" description="Inspect Leader and Helper infrastructure." actions={<Badge tone="accent">DEMO TOPOLOGY</Badge>} />
      <div className="demo-banner"><span className="demo-banner-mark">i</span><p><strong>Simulated node inventory</strong><span> These connection states are fixture values. No node heartbeat is being polled.</span></p><span className="banner-source">NOT LIVE</span></div>
      <section className="surface table-surface">
        <div className="table-toolbar"><SearchInput value={query} onChange={setQuery} placeholder="Search node, ID, or region" label="Search nodes" /><label className="select-control"><Filter size={14} /><select value={role} onChange={(event) => setRole(event.target.value)}>{selectOptions(['All roles', 'Leader', 'Helper'])}</select><ChevronDown size={13} /></label><span className="toolbar-spacer" /><span className="table-meta"><span className="status-dot good" />2 demo connected</span></div>
        <div className="table-wrap"><table className="data-table"><thead><tr><th>Node</th><th>Role</th><th>Status</th><th>Region</th><th>Version</th><th>Last heartbeat</th></tr></thead><tbody>{nodes.map((node) => <tr key={node.id} className="clickable-row" onClick={() => setSelected(node)}><td><div className="table-title-with-mark"><span className={`node-glyph small-glyph ${node.role === 'Leader' ? 'leader-glyph' : 'helper-glyph'}`}>{node.role[0]}</span><span><strong>{node.name}</strong><small className="mono">{node.id}</small></span></div></td><td>{node.role}</td><td><StateBadge value={node.status} /></td><td>{node.region}</td><td className="mono">{node.version}</td><td>{formatRelativeTime(node.heartbeat)} <span className="table-demo">DEMO</span></td></tr>)}{nodes.length === 0 && <tr><td colSpan={6}><EmptyState title="No nodes match" detail="Try another name, role, or region." /></td></tr>}</tbody></table></div>
        <div className="table-footer"><span>Node topology is <strong>not configured</strong></span><span>Values shown: demonstration fixtures</span></div>
      </section>
      <div className="node-summary-grid"><div className="surface summary-panel"><div className="summary-icon leader-summary"><span>L</span></div><div><small>LEADER ROLE</small><strong>Leader North</strong><span>Coordination origin · simulated</span></div><StateBadge value="Connected" /></div><div className="surface summary-panel"><div className="summary-icon helper-summary"><span>H</span></div><div><small>HELPER ROLE</small><strong>Helper West</strong><span>Peer coordination · simulated</span></div><StateBadge value="Connected" /></div></div>
      <Drawer open={selected !== null} onClose={() => setSelected(null)} title={selected?.name ?? 'Node detail'} eyebrow="DEMO NODE">
        {selected && <div className="detail-stack"><div className="detail-status"><span className={`node-glyph ${selected.role === 'Leader' ? 'leader-glyph' : 'helper-glyph'}`}>{selected.role[0]}</span><div><strong>{selected.role} role</strong><small>Simulated inventory record</small></div><StateBadge value={selected.status} /></div><div className="detail-group"><DetailRow label="Node ID" value={selected.id} mono /><DetailRow label="Host" value={selected.host} mono /><DetailRow label="Region" value={selected.region} /><DetailRow label="Version" value={selected.version} mono /><DetailRow label="Last heartbeat" value={formatDateTime(selected.heartbeat)} /></div><div className="timeline-mini"><strong>Connection timeline</strong><div><span className="timeline-dot good" /><p><b>Heartbeat observed</b><small>{formatRelativeTime(selected.heartbeat)} · demo fixture</small></p></div><div><span className="timeline-dot neutral" /><p><b>mTLS status unavailable</b><small>Not configured for this workspace</small></p></div></div><div className="detail-callout"><Network size={16} /><p>{selected.detail}</p></div></div>}
      </Drawer>
    </div>
  );
}

export function JournalPage() {
  const [query, setQuery] = useState('');
  const [state, setState] = useState<MessageStatus | 'All stored states'>('All stored states');
  const [sender, setSender] = useState('All senders');
  const [recipient, setRecipient] = useState('All recipients');
  const [timeRange, setTimeRange] = useState<JournalTimeRange>('Any time');
  const [selected, setSelected] = useState<JournalRecord | null>(null);
  const [sortKey, setSortKey] = useState<JournalSortKey>('received');
  const [sortDirection, setSortDirection] = useState<JournalSortDirection>('desc');
  const notify = useToast();
  const records = useMemo(() => sortJournalRecords(
    filterJournalRecords(demoJournal, { query, state, sender, recipient, timeRange }),
    sortKey,
    sortDirection,
  ), [query, state, sender, recipient, timeRange, sortKey, sortDirection]);
  const hasFilters = Boolean(query.trim()) || state !== 'All stored states' || sender !== 'All senders' || recipient !== 'All recipients' || timeRange !== 'Any time';

  function sortBy(nextKey: JournalSortKey) {
    if (sortKey === nextKey) {
      setSortDirection((current) => current === 'asc' ? 'desc' : 'asc');
      return;
    }
    setSortKey(nextKey);
    setSortDirection(nextKey === 'state' ? 'asc' : 'desc');
  }

  function clearFilters() {
    setQuery('');
    setState('All stored states');
    setSender('All senders');
    setRecipient('All recipients');
    setTimeRange('Any time');
  }

  async function copyDigest() {
    if (!selected) return;
    try {
      await copyTextToClipboard(selected.digest);
      notify({ title: 'Digest copied', detail: 'The complete synthetic demo digest was copied.', tone: 'success' });
    } catch {
      notify({ title: 'Copy unavailable', detail: 'Clipboard access was denied by this browser.', tone: 'error' });
    }
  }
  return (
    <div className="page-stack">
      <PageHeader eyebrow="Communication / 01" title="Message Journal" description="Inspect demonstration receipts and processing states." actions={<Badge tone="accent">DEMO RECORDS</Badge>} />
      <div className="demo-banner"><span className="demo-banner-mark">i</span><p><strong>No live journal connection</strong><span> Rows below are synthetic. Verification fields are intentionally not asserted as valid.</span></p><span className="banner-source">SIMULATED</span></div>
      <section className="surface table-surface">
        <div className="journal-stat-strip"><div><small>STORED STATES</small><strong>{demoJournal.length}</strong></div><div><small>PENDING</small><strong>{demoJournal.filter((entry) => entry.state === 'RECEIVED_PENDING').length}</strong></div><div><small>PROCESSED</small><strong>{demoJournal.filter((entry) => entry.state === 'PROCESSED').length}</strong></div><div><small>REJECTED</small><strong>{demoJournal.filter((entry) => entry.state === 'REJECTED').length}</strong></div><span>Counts are demo fixtures</span></div>
        <div className="table-toolbar">
          <SearchInput value={query} onChange={setQuery} placeholder="Search message ID or task" label="Search journal" />
          <label className="select-control"><Filter size={14} /><select aria-label="Filter by state" value={state} onChange={(event) => setState(event.target.value as MessageStatus | 'All stored states')}>{selectOptions(['All stored states', 'RECEIVED_PENDING', 'PROCESSED', 'REJECTED'])}</select><ChevronDown size={13} /></label>
          <label className="select-control"><select aria-label="Filter by sender" value={sender} onChange={(event) => setSender(event.target.value)}>{selectOptions(['All senders', ...new Set(demoJournal.map((message) => message.sender))])}</select><ChevronDown size={13} /></label>
          <label className="select-control"><select aria-label="Filter by recipient" value={recipient} onChange={(event) => setRecipient(event.target.value)}>{selectOptions(['All recipients', ...new Set(demoJournal.map((message) => message.recipient))])}</select><ChevronDown size={13} /></label>
          <label className="select-control"><Clock3 size={14} /><select aria-label="Filter by time" value={timeRange} onChange={(event) => setTimeRange(event.target.value as JournalTimeRange)}>{selectOptions(['Any time', 'Last hour', 'Last 24 hours', 'Last 7 days'])}</select><ChevronDown size={13} /></label>
          {hasFilters && <Button className="clear-filter-button" variant="quiet" onClick={clearFilters}><X size={14} />Clear filters</Button>}
          <span className="table-footnote-inline">NOT_RECEIVED means no record; it is not shown as a row.</span>
        </div>
        <div className="table-wrap"><table className="data-table journal-table"><thead><tr><th scope="col">Message ID</th><th scope="col">Task</th><th scope="col">Direction</th><JournalSortHeader label="Sequence" sortKey="sequence" activeKey={sortKey} direction={sortDirection} onSort={sortBy} /><JournalSortHeader label="Received" sortKey="received" activeKey={sortKey} direction={sortDirection} onSort={sortBy} /><JournalSortHeader label="State" sortKey="state" activeKey={sortKey} direction={sortDirection} onSort={sortBy} /></tr></thead><tbody>{records.map((message) => <tr key={message.id} className="clickable-row journal-row" tabIndex={0} aria-label={`Open journal details for ${message.id}`} onClick={() => setSelected(message)} onKeyDown={(event) => { if (event.target === event.currentTarget && (event.key === 'Enter' || event.key === ' ')) { event.preventDefault(); setSelected(message); } }}><td><button className="table-primary mono" onClick={(event) => { event.stopPropagation(); setSelected(message); }}>{message.id}<small>SHA-256 · {message.digest.slice(0, 8)}…</small></button></td><td className="mono">{message.taskId}</td><td><span className="direction-cell">{message.sender}<ChevronRight size={12} />{message.recipient}</span></td><td className="mono">{message.sequence.toLocaleString()}</td><td><time dateTime={message.receivedAt}>{formatRelativeTime(message.receivedAt)}</time> <span className="table-demo">DEMO</span></td><td><StateBadge value={message.state} /></td></tr>)}{records.length === 0 && <tr><td colSpan={6}><EmptyState title={demoJournal.length === 0 ? 'No stored journal records' : 'No messages match these filters'} detail={demoJournal.length === 0 ? 'NOT_RECEIVED means no journal record exists.' : 'Clear filters or try a different message, task, sender, recipient, state, or time range.'} /></td></tr>}</tbody></table></div>
        <div className="table-footer"><span>Showing <strong>{records.length}</strong> of {demoJournal.length} demo records</span><span>Digest display is truncated</span></div>
      </section>
      <Drawer open={selected !== null} onClose={() => setSelected(null)} title={selected?.id ?? 'Message detail'} eyebrow="DEMONSTRATION RECEIPT">
        {selected && <div className="detail-stack">
          <div className="detail-status"><span className="node-glyph journal-glyph"><Database size={16} /></span><div><strong>Current journal state</strong><small>Demonstration fixture · not a live record</small></div><StateBadge value={selected.state} /></div>
          <div className="detail-group">
            <DetailRow label="Full message ID" value={selected.id} mono />
            <DetailRow label="Task ID" value={selected.taskId} mono />
            <DetailRow label="Sender" value={selected.sender} mono />
            <DetailRow label="Recipient" value={selected.recipient} mono />
            <DetailRow label="Direction" value={<span className="journal-direction-full">{selected.sender}<ChevronRight size={13} />{selected.recipient}</span>} />
            <DetailRow label="Sequence number" value={selected.sequence.toLocaleString()} mono />
            <DetailRow label="Protocol version" value={selected.protocolVersion} mono />
            <DetailRow label="Received" value={formatDateTime(selected.receivedAt)} />
          </div>
          <section className="journal-evidence-section" aria-labelledby="journal-digest-heading">
            <div className="journal-section-heading"><h3 id="journal-digest-heading">SHA-256 digest</h3><span>SYNTHETIC DEMO VALUE</span></div>
            <div className="journal-digest-box"><code className="mono">{selected.digest}</code><IconButton icon={Copy} label="Copy full SHA-256 digest" onClick={() => void copyDigest()} /></div>
          </section>
          {(selected.processingDetail || selected.rejectionDetail) && <section className="journal-evidence-section"><div className="journal-section-heading"><h3>{selected.state === 'REJECTED' ? 'Rejection details' : 'Processing details'}</h3><Badge tone="neutral">DEMO ONLY</Badge></div><p className="journal-detail-copy">{selected.rejectionDetail ?? selected.processingDetail}</p></section>}
          <section className="journal-evidence-section" aria-labelledby="journal-auth-heading">
            <div className="journal-section-heading"><h3 id="journal-auth-heading">Authentication evidence</h3><span>NO LIVE BACKEND RESULT</span></div>
            <VerificationEvidence label="HMAC verification" status={selected.hmacStatus} detail="This demonstration record has no backend verification result." />
            <VerificationEvidence label="Sender credential binding" status={selected.senderAuthenticationStatus} detail="Authenticated sender credentials are not connected to this journal view." />
          </section>
          <div className="detail-callout warning-callout"><ShieldAlert size={16} /><p>This row is synthetic. The full digest is a display fixture, not a computed or verified record digest. No HMAC secret, MAC, or private credential is shown.</p></div>
        </div>}
      </Drawer>
    </div>
  );
}

function JournalSortHeader({ label, sortKey, activeKey, direction, onSort }: { label: string; sortKey: JournalSortKey; activeKey: JournalSortKey; direction: JournalSortDirection; onSort: (key: JournalSortKey) => void }) {
  const active = sortKey === activeKey;
  const Icon = !active ? ArrowUpDown : direction === 'asc' ? ArrowUp : ArrowDown;
  return <th scope="col" aria-sort={active ? direction === 'asc' ? 'ascending' : 'descending' : 'none'}><button className="table-sort" onClick={() => onSort(sortKey)} aria-label={`Sort by ${label}${active ? `, ${direction === 'asc' ? 'ascending' : 'descending'}` : ''}`}><span>{label}</span><Icon size={13} aria-hidden="true" /></button></th>;
}

function VerificationEvidence({ label, status, detail }: { label: string; status: JournalRecord['hmacStatus']; detail: string }) {
  const tone = status === 'Verified' ? 'good' : status === 'Unverified' ? 'warning' : 'neutral';
  return <div className="verification-row"><div><strong>{label}</strong><small>{detail}</small></div><Badge tone={tone}>{status}</Badge></div>;
}

export function ActivityPage() {
  const [query, setQuery] = useState('');
  const [category, setCategory] = useState('All categories');
  const [severity, setSeverity] = useState('All severities');
  const [timeRange, setTimeRange] = useState('Any time');
  const [selected, setSelected] = useState<ActivityRecord | null>(null);
  const cutoff = timeRange === 'Any time' ? 0 : Date.now() - ({ 'Last hour': 60, 'Last 24 hours': 1440, 'Last 7 days': 10080 }[timeRange] ?? 0) * 60_000;
  const filtered = demoActivity.filter((event) =>
    (category === 'All categories' || event.category === category)
    && (severity === 'All severities' || event.status === severity)
    && Date.parse(event.timestamp) >= cutoff
    && filterByQuery([event], query, ['event', 'source', 'detail', 'category'])[0],
  );
  return (
    <div className="page-stack">
      <PageHeader eyebrow="Communication / 02" title="Activity" description="A chronological view of demonstration system events." actions={<Badge tone="accent">SYNTHETIC FEED</Badge>} />
      <section className="surface table-surface">
        <div className="table-toolbar">
          <SearchInput value={query} onChange={setQuery} placeholder="Search event or source" label="Search activity" />
          <label className="select-control"><Filter size={14} /><select aria-label="Filter by category" value={category} onChange={(event) => setCategory(event.target.value)}>{selectOptions(['All categories', 'Task', 'Communication', 'Node', 'Security', 'Journal', 'System'])}</select><ChevronDown size={13} /></label>
          <label className="select-control"><Shield size={14} /><select aria-label="Filter by severity" value={severity} onChange={(event) => setSeverity(event.target.value)}>{selectOptions(['All severities', 'Success', 'Warning', 'Failed', 'Info'])}</select><ChevronDown size={13} /></label>
          <label className="select-control"><Clock3 size={14} /><select aria-label="Filter by time" value={timeRange} onChange={(event) => setTimeRange(event.target.value)}>{selectOptions(['Any time', 'Last hour', 'Last 24 hours', 'Last 7 days'])}</select><ChevronDown size={13} /></label>
          <span className="toolbar-spacer" /><span className="table-meta">{filtered.length} events</span>
        </div>
        <div className="activity-timeline">
          {filtered.map((event, index) => <ActivityItem key={event.id} event={event} first={index === 0} onSelect={setSelected} />)}
          {filtered.length === 0 && <EmptyState title="No events found" detail="Adjust the category or search query." />}
        </div>
        <div className="table-footer"><span>All events are demonstration fixtures</span><span>Times shown in local timezone</span></div>
      </section>
      <Drawer open={selected !== null} onClose={() => setSelected(null)} title={selected?.event ?? 'Activity detail'} eyebrow="DEMONSTRATION EVENT">
        {selected && <div className="detail-stack"><div className="detail-status"><span className="node-glyph journal-glyph"><Activity size={15} /></span><div><strong>{selected.category} event</strong><small>Fixture value · not a verified alert</small></div><StateBadge value={selected.status} /></div><div className="detail-group"><DetailRow label="Event" value={selected.event} /><DetailRow label="Source" value={selected.source} /><DetailRow label="Category" value={selected.category} /><DetailRow label="Status" value={selected.status} /><DetailRow label="Timestamp" value={formatDateTime(selected.timestamp)} /><DetailRow label="Event ID" value={selected.id} mono /><DetailRow label="Details" value={selected.detail} /></div></div>}
      </Drawer>
    </div>
  );
}

function ActivityItem({ event, first, onSelect }: { event: ActivityRecord; first: boolean; onSelect: (event: ActivityRecord) => void }) {
  return <article className={`timeline-item ${first ? 'timeline-first' : ''}`} role="button" tabIndex={0} aria-label={`View details for ${event.event}`} onClick={() => onSelect(event)} onKeyDown={(keyboardEvent) => { if (keyboardEvent.key === 'Enter' || keyboardEvent.key === ' ') { keyboardEvent.preventDefault(); onSelect(event); } }}><span className={`timeline-icon timeline-${event.status.toLowerCase()}`}><Activity size={14} /></span><div className="timeline-body"><div className="timeline-head"><strong>{event.event}</strong><StateBadge value={event.status} /></div><p>{event.detail}</p><div className="timeline-meta"><span>{event.source}</span><span>{event.category}</span><span>{formatRelativeTime(event.timestamp)}</span><span className="table-demo">DEMO</span></div></div><span className="timeline-time">{formatDateTime(event.timestamp)}</span></article>;
}

export function SecurityPage() {
  const implemented = securityControls.filter((control) => control.state === 'Implemented' || control.state === 'Verified').length;
  return (
    <div className="page-stack">
      <PageHeader eyebrow="System / 01" title="Security" description="Approved policy and the current implementation boundary." actions={<Badge tone="warning"><span className="badge-dot" />NOT DEPLOYMENT VERIFIED</Badge>} />
      <div className="security-summary surface"><div className="security-summary-icon"><ShieldCheck size={19} /></div><div><strong>Policy approved · enforcement incomplete</strong><p>A–H define the target design. Approval is not proof of implementation, testing, independent review, or deployment.</p></div><div className="security-summary-count"><strong>{implemented}<span> / {securityControls.length}</span></strong><small>implemented or verified controls</small></div></div>
      <section className="surface table-surface security-table-surface">
        <div className="panel-heading"><SectionTitle title="Control status" detail="Demonstration inventory derived from repository evidence" /><span className="table-demo">FIXTURE</span></div>
        <div className="security-control-list">{securityControls.map((control) => <SecurityControlRow key={control.id} control={control} />)}</div>
      </section>
      <div className="security-bottom-grid"><section className="surface security-note"><div className="note-icon good-note"><Check size={16} /></div><div><span>RESOLVED FINDING</span><h2>JSA-01 · Transaction boundary</h2><p>The journal processing callback was removed. Its API exposes no SQLite handle, and regression tests cover failures and concurrent state transitions.</p></div></section><section className="surface security-note"><div className="note-icon warning-note"><ShieldAlert size={16} /></div><div><span>OPEN LIMITATION</span><h2>Recovery authenticity</h2><p>SHA-256 and mirrored metadata detect ordinary inconsistencies, not a coherently rewritten database. No HMAC recovery verification is claimed.</p></div></section></div>
      <section className="surface gates-panel"><div className="panel-heading"><SectionTitle title="Before coordination routes" detail="All gates must pass; routes remain disabled" /></div><div className="gate-list">{['Database uniqueness and sender credential binding', 'Durable replay, freshness, retention, and recovery', 'OS-protected keys, database files, backups, and IPC', 'mTLS certificate lifecycle and peer authorization', 'Independent parser review and proxy/backend framing tests'].map((gate, index) => <div className="gate-row" key={gate}><span className="gate-index">0{index + 1}</span><span>{gate}</span><StateBadge value="Planned" /></div>)}</div></section>
    </div>
  );
}

function SecurityControlRow({ control }: { control: SecurityControl }) {
  const implementation = control.state === 'Implemented' || control.state === 'Verified' ? control.state : control.state === 'Not Configured' || control.state === 'Unavailable' ? control.state : 'Planned';
  return <div className="security-control-row"><span className={`security-control-icon state-${implementation.toLowerCase().replaceAll(' ', '-')}`}>{implementation === 'Verified' ? <Check size={15} /> : implementation === 'Not Configured' ? <X size={15} /> : <Clock3 size={15} />}</span><div className="security-control-main"><div className="security-control-title"><strong>{control.title}</strong><span>{control.area}</span></div><p>{control.detail}</p></div><div className="security-control-state"><Badge tone="accent">POLICY APPROVED</Badge><StateBadge value={implementation} /></div></div>;
}

export function SettingsPage() {
  const initialUrl = import.meta.env.VITE_API_BASE_URL ?? '';
  const [baseUrl, setBaseUrl] = useState(initialUrl);
  const [savedUrl, setSavedUrl] = useState(initialUrl);
  const [checkState, setCheckState] = useState<'idle' | 'checking' | 'connected' | 'error'>('idle');
  const [checkMessage, setCheckMessage] = useState('');
  const [emailNotices, setEmailNotices] = useState(false);
  const [digestNotices, setDigestNotices] = useState(true);
  const [compactDensity, setCompactDensity] = useState(false);
  const notify = useToast();

  useEffect(() => {
    document.documentElement.dataset.tableDensity = compactDensity ? 'compact' : 'comfortable';
    return () => { delete document.documentElement.dataset.tableDensity; };
  }, [compactDensity]);

  function saveEndpoint(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const value = baseUrl.trim().replace(/\/$/, '');
    if (value) {
      try { new URL(value); } catch { setCheckMessage('Enter a valid absolute URL, including http:// or https://.'); setCheckState('error'); return; }
    }
    setSavedUrl(value);
    setBaseUrl(value);
    setCheckState('idle');
    setCheckMessage('Saved for this page session only. No credential is stored.');
    notify({ title: 'Endpoint preference updated', detail: 'This value is held in memory and is not persisted to browser storage.', tone: 'success' });
  }

  async function testEndpoint() {
    setCheckState('checking');
    setCheckMessage('Contacting the confirmed FastAPI /healthz endpoint…');
    try {
      const health = await blindMarkApi.checkHealth(savedUrl);
      setCheckState('connected');
      setCheckMessage(`Liveness response received: ${health.service} (${health.status}). This does not verify aggregator coordination or security controls.`);
    } catch (error) {
      setCheckState('error');
      setCheckMessage(error instanceof ApiRequestError ? error.failure.message : 'Unexpected health-check error.');
    }
  }

  return (
    <div className="page-stack">
      <PageHeader eyebrow="System / 02" title="Settings" description="Workspace preferences and the confirmed API connection boundary." actions={<Badge tone="accent">LOCAL PREVIEW</Badge>} />
      <div className="settings-layout">
        <div className="settings-main">
          <section className="surface settings-section"><div className="settings-section-heading"><span className="settings-section-icon"><Network size={17} /></span><div><h2>Connection</h2><p>Only the confirmed FastAPI liveness endpoint is probed.</p></div><Badge tone={savedUrl ? 'info' : 'neutral'}>{savedUrl ? 'CONFIGURED' : 'NOT CONFIGURED'}</Badge></div>
            <form className="settings-form" onSubmit={saveEndpoint}><label className="form-field"><span>API base URL</span><input type="url" inputMode="url" value={baseUrl} onChange={(event) => setBaseUrl(event.target.value)} placeholder="https://api.example.test" autoComplete="url" /><small>Health probe: <code>/healthz</code>. No admin or service credentials are requested or stored.</small></label><div className="settings-form-actions"><Button type="submit" variant="primary">Save endpoint</Button><Button type="button" disabled={!savedUrl || checkState === 'checking'} onClick={testEndpoint}>{checkState === 'checking' ? <RefreshCw className="spin" size={15} /> : <Activity size={15} />}{checkState === 'checking' ? 'Checking' : 'Test health endpoint'}</Button></div>{checkMessage && <div className={`connection-result result-${checkState}`} role="status"><span className="connection-result-dot" />{checkMessage}</div>}</form>
          </section>
          <section className="surface settings-section"><div className="settings-section-heading"><span className="settings-section-icon"><SlidersHorizontal size={17} /></span><div><h2>Workspace preferences</h2><p>Changes apply to this page session only.</p></div></div><div className="setting-row"><div><strong>Compact data tables</strong><small>Reduce row height on wide screens.</small></div><button className={`toggle ${compactDensity ? 'toggle-on' : ''}`} role="switch" aria-checked={compactDensity} aria-label="Compact data tables" onClick={() => setCompactDensity((value) => !value)}><span /></button></div><div className="setting-row"><div><strong>Operational notifications</strong><small>Show local demonstration notices.</small></div><button className={`toggle ${emailNotices ? 'toggle-on' : ''}`} role="switch" aria-checked={emailNotices} aria-label="Operational notifications" onClick={() => setEmailNotices((value) => !value)}><span /></button></div><div className="setting-row"><div><strong>Daily summary</strong><small>Demo preference; no messages are sent.</small></div><button className={`toggle ${digestNotices ? 'toggle-on' : ''}`} role="switch" aria-checked={digestNotices} aria-label="Daily summary" onClick={() => setDigestNotices((value) => !value)}><span /></button></div></section>
          <section className="surface settings-section"><div className="settings-section-heading"><span className="settings-section-icon"><KeyRound size={17} /></span><div><h2>Credentials</h2><p>Secret management is not available in this frontend.</p></div><Badge tone="warning">UNAVAILABLE</Badge></div><div className="credentials-note"><ShieldAlert size={17} /><div><strong>No secret fields are exposed</strong><p>Do not enter API keys, HMAC secrets, certificates, or private keys here. Secure credential storage requires backend and deployment support.</p></div></div></section>
        </div>
        <aside className="settings-aside"><section className="surface settings-aside-card"><span className="settings-aside-kicker">ABOUT THIS WORKSPACE</span><div className="about-mark"><span className="brand-mark"><span /><span /><span /></span><strong>BlindMark</strong></div><p>Secure Coordination</p><div className="detail-group"><DetailRow label="Version" value="0.1.0-demo" mono /><DetailRow label="Environment" value="Local preview" /><DetailRow label="API endpoint" value={savedUrl || 'Not configured'} mono /></div><span className="about-demo-foot"><span className="demo-dot" />No live coordination</span></section><section className="surface settings-aside-card"><span className="settings-aside-kicker">SERVICE BOUNDARY</span><h3>Current API contract</h3><p>FastAPI exposes campaign metadata and a public <code>/healthz</code> liveness probe. The Rust service is separate; its report-ingestion route returns 501. No task, node, or journal UI APIs are connected.</p><span className="api-contract-link">Health probe only</span></section></aside>
      </div>
    </div>
  );
}
