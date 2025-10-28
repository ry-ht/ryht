import type { RouteObject } from 'react-router';

import { Outlet } from 'react-router';
import { lazy, Suspense } from 'react';

import { DashboardContent, DashboardLayout } from 'src/layouts/dashboard';

import { LoadingScreen } from 'src/components/loading-screen';

import { usePathname } from '../hooks';

// ----------------------------------------------------------------------

const IndexPage = lazy(() => import('src/pages/dashboard/overview'));

// Agent pages
const AgentListPage = lazy(() => import('src/pages/dashboard/agents/list'));
const AgentCreatePage = lazy(() => import('src/pages/dashboard/agents/create'));
const AgentDetailPage = lazy(() => import('src/pages/dashboard/agents/detail'));

// Agent type pages
const AgentOrchestratorPage = lazy(() => import('src/pages/dashboard/agents/type/orchestrator'));
const AgentDeveloperPage = lazy(() => import('src/pages/dashboard/agents/type/developer'));
const AgentReviewerPage = lazy(() => import('src/pages/dashboard/agents/type/reviewer'));
const AgentTesterPage = lazy(() => import('src/pages/dashboard/agents/type/tester'));
const AgentDocumenterPage = lazy(() => import('src/pages/dashboard/agents/type/documenter'));
const AgentArchitectPage = lazy(() => import('src/pages/dashboard/agents/type/architect'));
const AgentResearcherPage = lazy(() => import('src/pages/dashboard/agents/type/researcher'));
const AgentOptimizerPage = lazy(() => import('src/pages/dashboard/agents/type/optimizer'));

// Workflow pages
const WorkflowListPage = lazy(() => import('src/pages/dashboard/workflows/list'));
const WorkflowCreatePage = lazy(() => import('src/pages/dashboard/workflows/create'));
const WorkflowDetailPage = lazy(() => import('src/pages/dashboard/workflows/detail'));
const WorkflowTemplatesPage = lazy(() => import('src/pages/dashboard/workflows/templates'));

// Task pages
const TaskListPage = lazy(() => import('src/pages/dashboard/tasks/list'));
const TaskDetailPage = lazy(() => import('src/pages/dashboard/tasks/detail'));

// Coordination pages
const CoordinationMessagesPage = lazy(() => import('src/pages/dashboard/coordination/messages'));
const CoordinationSessionsPage = lazy(() => import('src/pages/dashboard/coordination/sessions'));
const CoordinationLocksPage = lazy(() => import('src/pages/dashboard/coordination/locks'));

// Memory pages
const MemoryWorkingPage = lazy(() => import('src/pages/dashboard/memory/working'));
const MemoryEpisodicPage = lazy(() => import('src/pages/dashboard/memory/episodic'));
const MemorySemanticPage = lazy(() => import('src/pages/dashboard/memory/semantic'));
const MemoryPatternsPage = lazy(() => import('src/pages/dashboard/memory/patterns'));
const MemoryConsolidationPage = lazy(() => import('src/pages/dashboard/memory/consolidation'));

// Code intelligence pages
const CodeAnalysisPage = lazy(() => import('src/pages/dashboard/code/analysis'));
const CodeDependenciesPage = lazy(() => import('src/pages/dashboard/code/dependencies'));
const CodeQualityPage = lazy(() => import('src/pages/dashboard/code/quality'));
const CodeVfsPage = lazy(() => import('src/pages/dashboard/code/vfs'));

// Knowledge pages
const KnowledgeDocumentsPage = lazy(() => import('src/pages/dashboard/knowledge/documents'));
const KnowledgeSearchPage = lazy(() => import('src/pages/dashboard/knowledge/search'));

// Monitoring pages
const MonitoringMetricsPage = lazy(() => import('src/pages/dashboard/monitoring/metrics'));
const MonitoringLogsPage = lazy(() => import('src/pages/dashboard/monitoring/logs'));
const MonitoringHealthPage = lazy(() => import('src/pages/dashboard/monitoring/health'));

// Config page
const ConfigPage = lazy(() => import('src/pages/dashboard/config'));

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
    <DashboardContent> 
      <SuspenseOutlet />
    </DashboardContent>
  </DashboardLayout>
);

export const dashboardRoutes: RouteObject[] = [
  {
    path: '/',
    element: dashboardLayout(),
    children: [
      { element: <IndexPage />, index: true },
      {
        path: 'agents',
        children: [
          { element: <AgentListPage />, index: true },
          { path: 'create', element: <AgentCreatePage /> },
          { path: ':id', element: <AgentDetailPage /> },
          { path: 'type/orchestrator', element: <AgentOrchestratorPage /> },
          { path: 'type/developer', element: <AgentDeveloperPage /> },
          { path: 'type/reviewer', element: <AgentReviewerPage /> },
          { path: 'type/tester', element: <AgentTesterPage /> },
          { path: 'type/documenter', element: <AgentDocumenterPage /> },
          { path: 'type/architect', element: <AgentArchitectPage /> },
          { path: 'type/researcher', element: <AgentResearcherPage /> },
          { path: 'type/optimizer', element: <AgentOptimizerPage /> },
        ],
      },
      {
        path: 'workflows',
        children: [
          { element: <WorkflowListPage />, index: true },
          { path: 'create', element: <WorkflowCreatePage /> },
          { path: 'templates', element: <WorkflowTemplatesPage /> },
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
        path: 'coordination',
        children: [
          { path: 'messages', element: <CoordinationMessagesPage /> },
          { path: 'sessions', element: <CoordinationSessionsPage /> },
          { path: 'locks', element: <CoordinationLocksPage /> },
        ],
      },
      {
        path: 'memory',
        children: [
          { path: 'working', element: <MemoryWorkingPage /> },
          { path: 'episodic', element: <MemoryEpisodicPage /> },
          { path: 'semantic', element: <MemorySemanticPage /> },
          { path: 'patterns', element: <MemoryPatternsPage /> },
          { path: 'consolidation', element: <MemoryConsolidationPage /> },
        ],
      },
      {
        path: 'code',
        children: [
          { path: 'analysis', element: <CodeAnalysisPage /> },
          { path: 'dependencies', element: <CodeDependenciesPage /> },
          { path: 'quality', element: <CodeQualityPage /> },
          { path: 'vfs', element: <CodeVfsPage /> },
        ],
      },
      {
        path: 'knowledge',
        children: [
          { path: 'documents', element: <KnowledgeDocumentsPage /> },
          { path: 'search', element: <KnowledgeSearchPage /> },
        ],
      },
      {
        path: 'monitoring',
        children: [
          { path: 'metrics', element: <MonitoringMetricsPage /> },
          { path: 'logs', element: <MonitoringLogsPage /> },
          { path: 'health', element: <MonitoringHealthPage /> },
        ],
      },
      { path: 'config', element: <ConfigPage /> },
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
