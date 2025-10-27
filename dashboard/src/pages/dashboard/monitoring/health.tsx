import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { MonitoringHealthView } from 'src/sections/monitoring/monitoring-health-view';

// ----------------------------------------------------------------------

const metadata = { title: `System Health - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <MonitoringHealthView />;
}
