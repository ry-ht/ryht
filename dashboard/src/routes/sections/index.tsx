import type { RouteObject } from 'react-router';

import { lazy } from 'react';

import { dashboardRoutes } from './dashboard';

// ----------------------------------------------------------------------

const Page404 = lazy(() => import('src/pages/error/404'));

export const routesSection: RouteObject[] = [
  // Dashboard (now at root path '/')
  ...dashboardRoutes,

  // No match
  { path: '*', element: <Page404 /> },
];
