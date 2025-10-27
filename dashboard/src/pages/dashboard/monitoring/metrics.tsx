import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { MonitoringMetricsView } from 'src/sections/monitoring/monitoring-metrics-view';

// ----------------------------------------------------------------------

const metadata = { title: `System Metrics - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <MonitoringMetricsView />;
}
