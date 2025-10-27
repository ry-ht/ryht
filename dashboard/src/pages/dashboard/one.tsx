import { CONFIG } from 'src/global-config';

import { useDocumentTitle } from 'src/hooks/use-document-title';

import { DashboardOverview } from 'src/sections/overview/dashboard-overview';

// ----------------------------------------------------------------------

const metadata = { title: `Dashboard - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <DashboardOverview />;
}
