import { CONFIG } from 'src/global-config';

import { useDocumentTitle } from 'src/hooks/use-document-title';

import { TelemetryView } from 'src/sections/overview/telemetry-view';

// ----------------------------------------------------------------------

const metadata = { title: `Telemetry | Dashboard - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <TelemetryView />;
}
