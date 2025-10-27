import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { MonitoringLogsView } from 'src/sections/monitoring/monitoring-logs-view';

// ----------------------------------------------------------------------

const metadata = { title: `Event Logs - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <MonitoringLogsView />;
}
