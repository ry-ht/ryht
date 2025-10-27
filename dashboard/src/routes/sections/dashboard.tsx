import type { RouteObject } from 'react-router';

import { Outlet } from 'react-router';
import { lazy, Suspense } from 'react';

import { CONFIG } from 'src/global-config';
import { DashboardLayout } from 'src/layouts/dashboard';

import { LoadingScreen } from 'src/components/loading-screen';

import { AuthGuard } from 'src/auth/guard';

import { usePathname } from '../hooks';

// ----------------------------------------------------------------------

const IndexPage = lazy(() => import('src/pages/dashboard/one'));
const PageTwo = lazy(() => import('src/pages/dashboard/two'));
const PageThree = lazy(() => import('src/pages/dashboard/three'));
const PageFour = lazy(() => import('src/pages/dashboard/four'));
const PageFive = lazy(() => import('src/pages/dashboard/five'));
const PageSix = lazy(() => import('src/pages/dashboard/six'));

// Agent pages
const AgentListPage = lazy(() => import('src/pages/dashboard/agents/list'));
const AgentCreatePage = lazy(() => import('src/pages/dashboard/agents/create'));
const AgentDetailPage = lazy(() => import('src/pages/dashboard/agents/detail'));

// Workflow pages
const WorkflowListPage = lazy(() => import('src/pages/dashboard/workflows/list'));
const WorkflowCreatePage = lazy(() => import('src/pages/dashboard/workflows/create'));
const WorkflowDetailPage = lazy(() => import('src/pages/dashboard/workflows/detail'));

// Telemetry pages
const TelemetryPage = lazy(() => import('src/pages/dashboard/telemetry'));

// Task pages
const TaskListPage = lazy(() => import('src/pages/dashboard/tasks/list'));
const TaskDetailPage = lazy(() => import('src/pages/dashboard/tasks/detail'));

// Cortex pages
const CortexOverviewPage = lazy(() => import('src/pages/cortex/overview'));
const CortexWorkspaceListPage = lazy(() => import('src/pages/cortex/workspaces/list'));
const CortexWorkspaceCreatePage = lazy(() => import('src/pages/cortex/workspaces/create'));
const CortexWorkspaceDetailPage = lazy(() => import('src/pages/cortex/workspaces/detail'));
const CortexWorkspaceBrowsePage = lazy(() => import('src/pages/cortex/workspaces/browse'));
const CortexWorkspaceCodeUnitsPage = lazy(() => import('src/pages/cortex/workspaces/code-units'));
const CortexWorkspaceDependenciesPage = lazy(() => import('src/pages/cortex/workspaces/dependencies'));
const CortexDocumentListPage = lazy(() => import('src/pages/cortex/documents/list'));
const CortexDocumentCreatePage = lazy(() => import('src/pages/cortex/documents/create'));
const CortexDocumentViewPage = lazy(() => import('src/pages/cortex/documents/view'));
const CortexMemorySearchPage = lazy(() => import('src/pages/cortex/memory'));
const CortexMemoryEpisodesPage = lazy(() => import('src/pages/cortex/memory/episodes'));
const CortexMemoryPatternsPage = lazy(() => import('src/pages/cortex/memory/patterns'));
const CortexMemoryConsolidatePage = lazy(() => import('src/pages/cortex/memory/consolidate'));

// ----------------------------------------------------------------------

function SuspenseOutlet() {
  const pathname = usePathname();
  return (
    <Suspense key={pathname} fallback={<LoadingScreen />}>
      <Outlet />
    </Suspense>
  );
}

const dashboardLayout = () => (
  <DashboardLayout>
    <SuspenseOutlet />
  </DashboardLayout>
);

export const dashboardRoutes: RouteObject[] = [
  {
    path: 'dashboard',
    element: CONFIG.auth.skip ? dashboardLayout() : <AuthGuard>{dashboardLayout()}</AuthGuard>,
    children: [
      { element: <IndexPage />, index: true },
      { path: 'two', element: <PageTwo /> },
      { path: 'three', element: <PageThree /> },
      {
        path: 'group',
        children: [
          { element: <PageFour />, index: true },
          { path: 'five', element: <PageFive /> },
          { path: 'six', element: <PageSix /> },
        ],
      },
      {
        path: 'agents',
        children: [
          { element: <AgentListPage />, index: true },
          { path: 'create', element: <AgentCreatePage /> },
        ],
      },
      {
        path: 'workflows',
        children: [
          { element: <WorkflowListPage />, index: true },
          { path: 'create', element: <WorkflowCreatePage /> },
          { path: ':id', element: <WorkflowDetailPage /> },
        ],
      },
      {
        path: 'tasks',
        children: [
          { element: <TaskListPage />, index: true },
          { path: ':id', element: <TaskDetailPage /> },
        ],
      },
      {
        path: 'cortex',
        children: [
          { element: <CortexOverviewPage />, index: true },
          {
            path: 'workspaces',
            children: [
              { element: <CortexWorkspaceListPage />, index: true },
              { path: 'create', element: <CortexWorkspaceCreatePage /> },
              { path: ':id', element: <CortexWorkspaceDetailPage /> },
              { path: ':id/browse', element: <CortexWorkspaceBrowsePage /> },
              { path: ':id/code-units', element: <CortexWorkspaceCodeUnitsPage /> },
              { path: ':id/dependencies', element: <CortexWorkspaceDependenciesPage /> },
            ],
          },
          {
            path: 'documents',
            children: [
              { element: <CortexDocumentListPage />, index: true },
              { path: 'create', element: <CortexDocumentCreatePage /> },
              { path: ':id', element: <CortexDocumentViewPage /> },
            ],
          },
          {
            path: 'memory',
            children: [
              { element: <CortexMemorySearchPage />, index: true },
              { path: 'episodes', element: <CortexMemoryEpisodesPage /> },
              { path: 'patterns', element: <CortexMemoryPatternsPage /> },
              { path: 'consolidate', element: <CortexMemoryConsolidatePage /> },
            ],
          },
        ],
      },
    ],
  },
];
