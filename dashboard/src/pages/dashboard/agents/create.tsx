import { Helmet } from 'react-helmet-async';

import { CONFIG } from 'src/global-config';

import { AgentCreateView } from 'src/sections/agent';

// ----------------------------------------------------------------------

const metadata = { title: `Create Agent - ${CONFIG.appName}` };

export default function Page() {
  return (
    <>
      <Helmet>
        <title>{metadata.title}</title>
      </Helmet>

      <AgentCreateView />
    </>
  );
}
