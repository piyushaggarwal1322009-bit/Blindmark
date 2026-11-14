import { useState } from 'react';
import { Link } from 'react-router-dom';
import { Activity, ArrowDownRight, ArrowUpRight, Check, ChevronRight, Clock3, Database, Network, ShieldCheck, Workflow } from 'lucide-react';
import { CartesianGrid, Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts';
import { demoActivity, demoNodes, demoSeries, demoTasks } from '../data/demo';
import { formatRelativeTime } from '../utils';
import { Badge, Drawer, PageHeader, ProgressBar, SectionTitle, StateBadge } from '../components/ui';
import type { NodeRecord } from '../types';

const metrics = [
  { title: 'Active tasks', value: '03', detail: 'of 06 demonstration tasks', icon: Workflow, trend: '+2', rising: true },
  { title: 'Connected nodes', value: '02', detail: 'of 03 demonstration nodes', icon: Network, trend: '1 unavailable', rising: false },
  { title: 'Messages processed', value: '1,842', detail: 'illustrative 24h volume', icon: Database, trend: '+12.4%', rising: true },
  { title: 'System health', value: '83%', detail: 'synthetic availability score', icon: ShieldCheck, trend: 'sample only', rising: false },
];

export default function Overview() {
  const [selectedNode, setSelectedNode] = useState<NodeRecord | null>(null);
  return (
    <div className="page-stack">
      <PageHeader eyebrow="Workspace / 01" title="Overview" description="Monitor your coordination infrastructure and system activity." actions={<><Badge tone="accent" className="demo-mode"><span className="demo-dot" />DEMO MODE</Badge><span className="date-range-static"><Clock3 size={15} />Last 24 hours</span></>} />
      <div className="demo-banner"><span className="demo-banner-mark">i</span><p><strong>Demonstration data</strong><span> Operational values and events below are simulated and are not verified live system information.</span></p><span className="banner-source">LOCAL FIXTURES</span></div>

      <section className="metric-grid" aria-label="Demonstration metrics">
        {metrics.map(({ title, value, detail, icon: Icon, trend, rising }) => (
          <article className="metric-card" key={title}>
            <div className="metric-top"><span>{title}</span><Icon size={17} strokeWidth={1.7} /></div>
            <div className="metric-main"><strong>{value}</strong><Badge tone="neutral" className="metric-demo">DEMO</Badge></div>
            <div className="metric-bottom"><span>{detail}</span><span className={`metric-trend ${rising ? 'trend-up' : ''}`}>{rising ? <ArrowUpRight size={13} /> : <ArrowDownRight size={13} />}{trend}</span></div>
          </article>
        ))}
      </section>

      <section className="surface topology-panel">
        <div className="panel-heading"><SectionTitle title="Coordination topology" detail="Simulated node status" /><span className="panel-updated"><span className="status-dot warning" />LIVE STATUS UNAVAILABLE</span></div>
        <div className="topology-content">
          <button className="node-visual" onClick={() => setSelectedNode(demoNodes[0])}>
            <span className="node-visual-top"><span className="node-glyph leader-glyph">L</span><span><strong>Leader North</strong><small>LEADER · node-leader-01</small></span><StateBadge value="Connected" /></span>
            <span className="node-visual-meta"><span>Heartbeat <b>{formatRelativeTime(demoNodes[0].heartbeat)}</b></span><span>Version <b className="mono">{demoNodes[0].version}</b></span></span>
            <span className="node-demo-note">Simulated fixture</span>
          </button>
          <div className="topology-link" aria-label="Simulated connection between nodes"><span /><span className="link-label">mTLS NOT CONFIGURED</span><span /></div>
          <button className="node-visual" onClick={() => setSelectedNode(demoNodes[1])}>
            <span className="node-visual-top"><span className="node-glyph helper-glyph">H</span><span><strong>Helper West</strong><small>HELPER · node-helper-01</small></span><StateBadge value="Connected" /></span>
            <span className="node-visual-meta"><span>Heartbeat <b>{formatRelativeTime(demoNodes[1].heartbeat)}</b></span><span>Version <b className="mono">{demoNodes[1].version}</b></span></span>
            <span className="node-demo-note">Simulated fixture</span>
          </button>
          <button className="node-visual node-visual-muted" onClick={() => setSelectedNode(demoNodes[2])}>
            <span className="node-visual-top"><span className="node-glyph reserve-glyph">H</span><span><strong>Helper Reserve</strong><small>HELPER · node-helper-02</small></span><StateBadge value="Unavailable" /></span>
            <span className="node-visual-meta"><span>Heartbeat <b>{formatRelativeTime(demoNodes[2].heartbeat)}</b></span><span>Version <b className="mono">{demoNodes[2].version}</b></span></span>
            <span className="node-demo-note">Simulated fixture</span>
          </button>
        </div>
      </section>

      <section className="surface chart-panel">
        <div className="panel-heading chart-heading"><SectionTitle title="Message throughput" detail="24-hour demonstration series" /><div className="chart-legend" aria-label="Chart series"><span><i className="legend-line received" />Received</span><span><i className="legend-line processed" />Processed</span><span><i className="legend-line rejected" />Rejected</span></div></div>
        <div className="chart-wrap" role="img" aria-label="Demonstration line chart showing received, processed, and rejected messages during the last 24 hours">
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={demoSeries} margin={{ top: 10, right: 12, bottom: 0, left: -18 }}>
              <CartesianGrid stroke="rgba(196,207,220,.08)" vertical={false} />
              <XAxis dataKey="time" axisLine={false} tickLine={false} tick={{ fill: '#788391', fontSize: 11 }} dy={10} />
              <YAxis axisLine={false} tickLine={false} tick={{ fill: '#788391', fontSize: 11 }} />
              <Tooltip contentStyle={{ background: '#171c23', border: '1px solid rgba(255,255,255,.1)', borderRadius: 8, color: '#edf2f6', fontSize: 12 }} labelStyle={{ color: '#9da8b4', marginBottom: 4 }} cursor={{ stroke: 'rgba(255,255,255,.15)' }} />
              <Line type="monotone" dataKey="received" name="Received" stroke="#79a9e8" strokeWidth={2} dot={false} activeDot={{ r: 4, strokeWidth: 0 }} />
              <Line type="monotone" dataKey="processed" name="Processed" stroke="#35c99a" strokeWidth={2} dot={false} activeDot={{ r: 4, strokeWidth: 0 }} />
              <Line type="monotone" dataKey="rejected" name="Rejected" stroke="#e7b45d" strokeWidth={1.8} dot={false} activeDot={{ r: 4, strokeWidth: 0 }} />
            </LineChart>
          </ResponsiveContainer>
        </div>
        <div className="chart-footnote"><span>Illustrative counts · not a live feed</span><span><span className="status-dot info" /> 2-hour intervals</span></div>
      </section>

      <div className="overview-lower-grid">
        <section className="surface compact-panel">
          <div className="panel-heading"><SectionTitle title="Recent tasks" detail="Demonstration workload" /><Link className="text-link" to="/tasks">All tasks <ChevronRight size={14} /></Link></div>
          <div className="task-list">
            {demoTasks.slice(0, 4).map((task) => (
              <Link to="/tasks" className="task-row" key={task.id}>
                <span className="task-symbol"><Workflow size={15} /></span>
                <span className="task-row-main"><strong>{task.name}</strong><small className="mono">{task.id}<span> · </span>{task.node}</small></span>
                <span className="task-row-status">{task.progress !== null && <span className="task-progress"><ProgressBar value={task.progress} />{task.progress}%</span>}<StateBadge value={task.status} /></span>
              </Link>
            ))}
          </div>
        </section>
        <section className="surface compact-panel">
          <div className="panel-heading"><SectionTitle title="Recent activity" detail="Synthetic event stream" /><Link className="text-link" to="/activity">View activity <ChevronRight size={14} /></Link></div>
          <div className="activity-list compact-activity">
            {demoActivity.slice(0, 5).map((event) => (
              <div className="activity-row" key={event.id}>
                <span className={`activity-marker activity-${event.status.toLowerCase()}`}>{event.status === 'Success' ? <Check size={11} /> : <Activity size={11} />}</span>
                <div className="activity-row-copy"><strong>{event.event}</strong><small>{event.source} <span>·</span> {formatRelativeTime(event.timestamp)}</small></div>
                <StateBadge value={event.status} />
              </div>
            ))}
          </div>
        </section>
      </div>

      <Drawer open={selectedNode !== null} onClose={() => setSelectedNode(null)} title={selectedNode?.name ?? 'Node details'} eyebrow="DEMO NODE">
        {selectedNode && <div className="detail-stack"><div className="detail-status"><span className={`node-glyph ${selectedNode.role === 'Leader' ? 'leader-glyph' : 'helper-glyph'}`}>{selectedNode.role[0]}</span><div><strong>{selectedNode.role} node</strong><small>Demonstration fixture · not a live connection</small></div><StateBadge value={selectedNode.status} /></div><div className="detail-group"><div className="detail-row"><span>Node identifier</span><strong className="mono">{selectedNode.id}</strong></div><div className="detail-row"><span>Host</span><strong className="mono">{selectedNode.host}</strong></div><div className="detail-row"><span>Region</span><strong>{selectedNode.region}</strong></div><div className="detail-row"><span>Version</span><strong className="mono">{selectedNode.version}</strong></div><div className="detail-row"><span>Last heartbeat</span><strong>{formatRelativeTime(selectedNode.heartbeat)}</strong></div></div><div className="detail-callout"><ShieldCheck size={16} /><p>{selectedNode.detail}</p></div></div>}
      </Drawer>
    </div>
  );
}
