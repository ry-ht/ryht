import { Helmet } from 'react-helmet-async';

import { CONFIG } from 'src/global-config';

import { AgentDetailView } from 'src/sections/agent/agent-detail-view';

// ----------------------------------------------------------------------

const metadata = { title: `Agent Details | Dashboard - ${CONFIG.appName}` };

export default function Page() {
  return (
    <>
      <Helmet>
        <title> {metadata.title}</title>
      </Helmet>

      <AgentDetailView />
    </>
  );
}
