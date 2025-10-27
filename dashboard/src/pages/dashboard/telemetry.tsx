import { Helmet } from 'react-helmet-async';

import { CONFIG } from 'src/global-config';

import { TelemetryView } from 'src/sections/overview/telemetry-view';

// ----------------------------------------------------------------------

const metadata = { title: `Telemetry | Dashboard - ${CONFIG.appName}` };

export default function Page() {
  return (
    <>
      <Helmet>
        <title> {metadata.title}</title>
      </Helmet>

      <TelemetryView />
    </>
  );
}
