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

// Workflow pages
const WorkflowListPage = lazy(() => import('src/pages/dashboard/workflows/list'));
const WorkflowCreatePage = lazy(() => import('src/pages/dashboard/workflows/create'));

// Cortex pages
const CortexOverviewPage = lazy(() => import('src/pages/cortex/overview'));
const CortexWorkspaceListPage = lazy(() => import('src/pages/cortex/workspaces/list'));
const CortexDocumentListPage = lazy(() => import('src/pages/cortex/documents/list'));
const CortexDocumentViewPage = lazy(() => import('src/pages/cortex/documents/view'));
const CortexMemorySearchPage = lazy(() => import('src/pages/cortex/memory'));

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
            ],
          },
          {
            path: 'documents',
            children: [
              { element: <CortexDocumentListPage />, index: true },
              { path: ':id', element: <CortexDocumentViewPage /> },
            ],
          },
          { path: 'memory', element: <CortexMemorySearchPage /> },
        ],
      },
    ],
  },
];
