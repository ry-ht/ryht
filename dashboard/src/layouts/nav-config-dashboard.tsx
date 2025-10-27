import type { NavSectionProps } from 'src/components/nav-section';

import { paths } from 'src/routes/paths';

import { CONFIG } from 'src/global-config';

import { Label } from 'src/components/label';
import { SvgColor } from 'src/components/svg-color';

// ----------------------------------------------------------------------

const icon = (name: string) => (
  <SvgColor src={`${CONFIG.assetsDir}/assets/icons/navbar/${name}.svg`} />
);

const ICONS = {
  job: icon('ic-job'),
  blog: icon('ic-blog'),
  chat: icon('ic-chat'),
  mail: icon('ic-mail'),
  user: icon('ic-user'),
  file: icon('ic-file'),
  lock: icon('ic-lock'),
  tour: icon('ic-tour'),
  order: icon('ic-order'),
  label: icon('ic-label'),
  blank: icon('ic-blank'),
  kanban: icon('ic-kanban'),
  folder: icon('ic-folder'),
  course: icon('ic-course'),
  params: icon('ic-params'),
  banking: icon('ic-banking'),
  booking: icon('ic-booking'),
  invoice: icon('ic-invoice'),
  product: icon('ic-product'),
  calendar: icon('ic-calendar'),
  disabled: icon('ic-disabled'),
  external: icon('ic-external'),
  subpaths: icon('ic-subpaths'),
  menuItem: icon('ic-menu-item'),
  ecommerce: icon('ic-ecommerce'),
  analytics: icon('ic-analytics'),
  dashboard: icon('ic-dashboard'),
};

// ----------------------------------------------------------------------

export const navData: NavSectionProps['data'] = [
  /**
   * System Overview
   */
  {
    subheader: 'Overview',
    items: [
      {
        title: 'Dashboard',
        path: paths.dashboard.root,
        icon: ICONS.dashboard,
        info: <Label color="info">RyHt</Label>,
      },
    ],
  },
  /**
   * Multi-Agent System (Axon)
   */
  {
    subheader: 'Multi-Agent System',
    items: [
      {
        title: 'Agents',
        path: paths.dashboard.agents.root,
        icon: ICONS.user,
        children: [
          { title: 'All Agents', path: paths.dashboard.agents.list },
          { title: 'Create Agent', path: paths.dashboard.agents.create },
          { title: 'Orchestrator', path: paths.dashboard.agents.byType('orchestrator') },
          { title: 'Developer', path: paths.dashboard.agents.byType('developer') },
          { title: 'Reviewer', path: paths.dashboard.agents.byType('reviewer') },
          { title: 'Tester', path: paths.dashboard.agents.byType('tester') },
          { title: 'Documenter', path: paths.dashboard.agents.byType('documenter') },
          { title: 'Architect', path: paths.dashboard.agents.byType('architect') },
          { title: 'Researcher', path: paths.dashboard.agents.byType('researcher') },
          { title: 'Optimizer', path: paths.dashboard.agents.byType('optimizer') },
        ],
      },
      {
        title: 'Workflows',
        path: paths.dashboard.workflows.root,
        icon: ICONS.kanban,
        children: [
          { title: 'All Workflows', path: paths.dashboard.workflows.list },
          { title: 'Create Workflow', path: paths.dashboard.workflows.create },
          { title: 'Templates', path: paths.dashboard.workflows.templates },
        ],
      },
      {
        title: 'Tasks',
        path: paths.dashboard.tasks.root,
        icon: ICONS.label,
      },
      {
        title: 'Coordination',
        path: paths.dashboard.coordination.root,
        icon: ICONS.chat,
        children: [
          { title: 'Messages', path: paths.dashboard.coordination.messages },
          { title: 'Sessions', path: paths.dashboard.coordination.sessions },
          { title: 'Locks', path: paths.dashboard.coordination.locks },
        ],
      },
    ],
  },
  /**
   * Cognitive System (Cortex)
   */
  {
    subheader: 'Cognitive System',
    items: [
      {
        title: 'Memory',
        path: paths.dashboard.memory.root,
        icon: ICONS.analytics,
        children: [
          { title: 'Working Memory', path: paths.dashboard.memory.working },
          { title: 'Episodic Memory', path: paths.dashboard.memory.episodic },
          { title: 'Semantic Memory', path: paths.dashboard.memory.semantic },
          { title: 'Patterns', path: paths.dashboard.memory.patterns },
          { title: 'Consolidation', path: paths.dashboard.memory.consolidation },
        ],
      },
      {
        title: 'Workspaces',
        path: paths.dashboard.workspaces.root,
        icon: ICONS.folder,
        children: [
          { title: 'All Workspaces', path: paths.dashboard.workspaces.list },
          { title: 'Create Workspace', path: paths.dashboard.workspaces.create },
        ],
      },
      {
        title: 'Code Intelligence',
        path: paths.dashboard.code.root,
        icon: ICONS.file,
        children: [
          { title: 'Code Analysis', path: paths.dashboard.code.analysis },
          { title: 'Dependencies', path: paths.dashboard.code.dependencies },
          { title: 'Quality Metrics', path: paths.dashboard.code.quality },
          { title: 'VFS Browser', path: paths.dashboard.code.vfs },
        ],
      },
      {
        title: 'Knowledge Base',
        path: paths.dashboard.knowledge.root,
        icon: ICONS.course,
        children: [
          { title: 'Documents', path: paths.dashboard.knowledge.documents },
          { title: 'Semantic Search', path: paths.dashboard.knowledge.search },
        ],
      },
    ],
  },
  /**
   * Monitoring & Analytics
   */
  {
    subheader: 'Monitoring',
    items: [
      {
        title: 'Real-time Metrics',
        path: paths.dashboard.monitoring.metrics,
        icon: ICONS.banking,
      },
      {
        title: 'Logs & Events',
        path: paths.dashboard.monitoring.logs,
        icon: ICONS.blog,
      },
      {
        title: 'System Health',
        path: paths.dashboard.monitoring.health,
        icon: ICONS.tour,
      },
    ],
  },
  /**
   * Configuration
   */
  {
    subheader: 'Settings',
    items: [
      {
        title: 'Configuration',
        path: paths.dashboard.config.root,
        icon: ICONS.params,
      },
    ],
  },
];
