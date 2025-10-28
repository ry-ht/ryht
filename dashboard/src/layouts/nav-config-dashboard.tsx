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
   * System
   */
  {
    subheader: 'System',
    items: [
      {
        title: 'Dashboard',
        path: paths.dashboard.root,
        icon: ICONS.dashboard,
      },
    ],
  },
  /**
   * Multi-Agent System
   */
  {
    subheader: 'Agents & Workflows',
    items: [
      {
        title: 'Agents',
        path: paths.dashboard.agents.root,
        icon: ICONS.user,
        children: [
          { title: 'All Agents', path: paths.dashboard.agents.list },
          { title: 'Create Agent', path: paths.dashboard.agents.create },
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
   * Cognitive System
   */
  {
    subheader: 'Cognitive System',
    items: [
      {
        title: 'Memory',
        path: paths.dashboard.memory.root,
        icon: ICONS.analytics,
        children: [
          { title: 'Working', path: paths.dashboard.memory.working },
          { title: 'Episodic', path: paths.dashboard.memory.episodic },
          { title: 'Semantic', path: paths.dashboard.memory.semantic },
          { title: 'Patterns', path: paths.dashboard.memory.patterns },
          { title: 'Consolidation', path: paths.dashboard.memory.consolidation },
        ],
      },
      {
        title: 'Workspaces',
        path: paths.cortex.workspaces.root,
        icon: ICONS.folder,
        children: [
          { title: 'All Workspaces', path: paths.cortex.workspaces.list },
          { title: 'Create', path: paths.cortex.workspaces.create },
        ],
      },
      {
        title: 'Code Analysis',
        path: paths.dashboard.code.root,
        icon: ICONS.file,
        children: [
          { title: 'Overview', path: paths.dashboard.code.analysis },
          { title: 'Dependencies', path: paths.dashboard.code.dependencies },
          { title: 'Quality', path: paths.dashboard.code.quality },
          { title: 'VFS Browser', path: paths.dashboard.code.vfs },
        ],
      },
      {
        title: 'Knowledge',
        path: paths.dashboard.knowledge.root,
        icon: ICONS.course,
        children: [
          { title: 'Documents', path: paths.dashboard.knowledge.documents },
          { title: 'Search', path: paths.dashboard.knowledge.search },
        ],
      },
    ],
  },
  /**
   * Monitoring
   */
  {
    subheader: 'Monitoring',
    items: [
      {
        title: 'Metrics',
        path: paths.dashboard.monitoring.metrics,
        icon: ICONS.banking,
      },
      {
        title: 'Logs',
        path: paths.dashboard.monitoring.logs,
        icon: ICONS.blog,
      },
      {
        title: 'Health',
        path: paths.dashboard.monitoring.health,
        icon: ICONS.tour,
      },
    ],
  },
  /**
   * Settings
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
