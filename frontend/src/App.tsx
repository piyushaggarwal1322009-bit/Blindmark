import { Component, lazy, Suspense, type ErrorInfo, type ReactNode } from 'react';
import { BrowserRouter, Route, Routes } from 'react-router-dom';
import { AlertTriangle } from 'lucide-react';
import Shell from './components/Shell';
import { ToastProvider } from './components/feedback';
import { EmptyState } from './components/ui';

const Overview = lazy(() => import('./pages/Overview'));
const TasksPage = lazy(() => import('./pages/Operations').then((module) => ({ default: module.TasksPage })));
const NodesPage = lazy(() => import('./pages/Operations').then((module) => ({ default: module.NodesPage })));
const JournalPage = lazy(() => import('./pages/Operations').then((module) => ({ default: module.JournalPage })));
const ActivityPage = lazy(() => import('./pages/Operations').then((module) => ({ default: module.ActivityPage })));
const SecurityPage = lazy(() => import('./pages/Operations').then((module) => ({ default: module.SecurityPage })));
const SettingsPage = lazy(() => import('./pages/Operations').then((module) => ({ default: module.SettingsPage })));

class ErrorBoundary extends Component<{ children: ReactNode }, { hasError: boolean }> {
  state = { hasError: false };

  static getDerivedStateFromError() {
    return { hasError: true };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('BlindMark demo UI error', error, info.componentStack);
  }

  render() {
    if (this.state.hasError) {
      return <div className="fatal-error"><AlertTriangle size={22} /><h1>Workspace unavailable</h1><p>The demonstration interface hit an unexpected error. Reload the page to try again.</p><button className="button button-secondary" onClick={() => window.location.reload()}>Reload workspace</button></div>;
    }
    return this.props.children;
  }
}

function NotFound() {
  return <div className="page-stack"><EmptyState title="Page not found" detail="That workspace route does not exist." /></div>;
}

function RouteFallback() {
  return <div className="page-stack route-fallback" role="status" aria-live="polite"><div className="skeleton-bar skeleton-title" /><div className="skeleton-bar skeleton-copy" /><div className="skeleton-panel" /></div>;
}

export default function App() {
  return (
    <ErrorBoundary>
      <ToastProvider>
        <BrowserRouter>
          <Suspense fallback={<RouteFallback />}>
            <Routes>
              <Route element={<Shell />}>
                <Route index element={<Overview />} />
                <Route path="tasks" element={<TasksPage />} />
                <Route path="nodes" element={<NodesPage />} />
                <Route path="journal" element={<JournalPage />} />
                <Route path="activity" element={<ActivityPage />} />
                <Route path="security" element={<SecurityPage />} />
                <Route path="settings" element={<SettingsPage />} />
                <Route path="*" element={<NotFound />} />
              </Route>
            </Routes>
          </Suspense>
        </BrowserRouter>
      </ToastProvider>
    </ErrorBoundary>
  );
}
