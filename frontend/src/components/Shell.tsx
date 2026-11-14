import { useEffect, useState } from 'react';
import { NavLink, Outlet, useLocation, useNavigate } from 'react-router-dom';
import { Activity, Bell, ChevronDown, ChevronLeft, Database, Gauge, LockKeyhole, Menu, Network, PanelLeftClose, PanelLeftOpen, Settings, Shield, Workflow, X } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { IconButton } from './ui';
import { useToast } from '../hooks/useToast';

interface NavItem { label: string; path: string; icon: LucideIcon; group: string }
const navItems: NavItem[] = [
  { label: 'Overview', path: '/', icon: Gauge, group: 'Workspace' },
  { label: 'Tasks', path: '/tasks', icon: Workflow, group: 'Workspace' },
  { label: 'Nodes', path: '/nodes', icon: Network, group: 'Workspace' },
  { label: 'Message Journal', path: '/journal', icon: Database, group: 'Communication' },
  { label: 'Activity', path: '/activity', icon: Activity, group: 'Communication' },
  { label: 'Security', path: '/security', icon: Shield, group: 'System' },
  { label: 'Settings', path: '/settings', icon: Settings, group: 'System' },
];
const pageNames: Record<string, string> = { '/': 'Overview', '/tasks': 'Tasks', '/nodes': 'Nodes', '/journal': 'Message Journal', '/activity': 'Activity', '/security': 'Security', '/settings': 'Settings' };

export default function Shell() {
  const [collapsed, setCollapsed] = useState(() => window.matchMedia('(max-width: 1200px)').matches);
  const [mobileOpen, setMobileOpen] = useState(false);
  const [notificationsOpen, setNotificationsOpen] = useState(false);
  const [profileOpen, setProfileOpen] = useState(false);
  const location = useLocation();
  const navigate = useNavigate();
  const notify = useToast();
  const pageTitle = pageNames[location.pathname] ?? 'Overview';

  useEffect(() => {
    const tabletLayout = window.matchMedia('(max-width: 1200px)');
    const handleLayoutChange = (event: MediaQueryListEvent) => {
      setCollapsed(event.matches);
      setMobileOpen(false);
    };
    tabletLayout.addEventListener('change', handleLayoutChange);
    return () => tabletLayout.removeEventListener('change', handleLayoutChange);
  }, []);

  function closeMobileNavigation() {
    setMobileOpen(false);
    setCollapsed(window.matchMedia('(max-width: 1200px)').matches);
  }

  return (
    <div className={`app-shell ${collapsed ? 'sidebar-collapsed' : ''}`}>
      {mobileOpen && <button className="mobile-scrim" aria-label="Close navigation" onClick={closeMobileNavigation} />}
      <aside className={`sidebar ${mobileOpen ? 'mobile-open' : ''}`}>
        <div className="brand-row">
          <span className="brand-mark" aria-hidden="true"><span /><span /><span /></span>
          {!collapsed && <span className="brand-word">BlindMark<span className="brand-product">SECURE COORDINATION</span></span>}
          {collapsed && <span className="sr-only">BlindMark Secure Coordination</span>}
          <button className="sidebar-collapse" aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'} title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'} onClick={() => setCollapsed((value) => !value)}>{collapsed ? <PanelLeftOpen size={16} /> : <PanelLeftClose size={16} />}</button>
          <button className="mobile-close icon-button" aria-label="Close navigation" onClick={closeMobileNavigation}><X size={17} /></button>
        </div>
        <div className="workspace-switcher" title="Demonstration workspace">
          <span className="workspace-avatar">BM</span>
          {!collapsed && <><span className="workspace-label"><strong>BlindMark Lab</strong><small>Demo workspace</small></span><ChevronDown size={14} /></>}
        </div>
        <nav className="primary-nav" aria-label="Primary navigation">
          {['Workspace', 'Communication', 'System'].map((group) => (
            <div className="nav-group" key={group}>
              {!collapsed && <span className="nav-heading">{group}</span>}
              {navItems.filter((item) => item.group === group).map(({ label, path, icon: Icon }) => (
                <NavLink key={path} to={path} end={path === '/'} className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`} title={collapsed ? label : undefined} onClick={() => { if (mobileOpen) closeMobileNavigation(); }}>
                  <Icon size={17} strokeWidth={1.8} /><span>{label}</span>{path === '/journal' && !collapsed && <span className="nav-count">2</span>}
                </NavLink>
              ))}
            </div>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="environment-card"><span className="environment-dot" />{!collapsed && <span><strong>DEMO ENVIRONMENT</strong><small>Isolated data</small></span>}</div>
          <button className="profile-row" onClick={() => setProfileOpen((value) => !value)} title={collapsed ? 'Demo operator' : undefined} aria-expanded={profileOpen}>
            <span className="profile-avatar">OP</span>{!collapsed && <><span className="profile-meta"><strong>Demo Operator</strong><small>Local preview</small></span><ChevronDown size={14} /></>}
          </button>
          {profileOpen && <div className="profile-popover"><strong>Demo Operator</strong><span>Authentication is not connected.</span><button onClick={() => { setProfileOpen(false); notify({ title: 'Sign-in unavailable', detail: 'No authentication flow is integrated in this demo.', tone: 'info' }); }}>Authentication status</button></div>}
        </div>
      </aside>

      <div className="main-column">
        <header className="topbar">
          <button className="mobile-menu icon-button" aria-label="Open navigation" onClick={() => { setCollapsed(false); setMobileOpen(true); }}><Menu size={18} /></button>
          <div className="breadcrumb"><span>BlindMark</span><ChevronLeft className="breadcrumb-chevron" size={13} /><strong>{pageTitle}</strong></div>
          <div className="topbar-spacer" />
          <div className="topbar-connection"><span className="connection-ring" /><span>Demo data</span></div>
          <div className="topbar-divider" />
          <div className="topbar-menu-wrap">
            <IconButton icon={Bell} label="Notifications" onClick={() => setNotificationsOpen((value) => !value)} aria-expanded={notificationsOpen} />
            {notificationsOpen && <div className="topbar-popover notification-popover"><div className="popover-heading"><strong>Notifications</strong><span>Demo</span></div><div className="notification-line"><span className="notification-mark warning" /><div><strong>One pending receipt</strong><small>Illustrative journal fixture · 7m ago</small></div></div><div className="notification-line"><span className="notification-mark neutral" /><div><strong>Security review gate open</strong><small>Independent parser review required</small></div></div><button className="popover-action" onClick={() => { setNotificationsOpen(false); navigate('/activity'); }}>View activity</button></div>}
          </div>
          <span className="topbar-user">OP</span>
        </header>
        <main className="main-content" id="main-content" key={location.pathname}>
          <Outlet />
          <footer className="app-footer"><span><LockKeyhole size={12} /> Demonstration workspace · no live coordination</span><span>BlindMark <b>0.1.0</b></span></footer>
        </main>
      </div>
    </div>
  );
}
