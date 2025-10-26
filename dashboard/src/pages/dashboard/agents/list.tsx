import { Helmet } from 'react-helmet-async';

import { CONFIG } from 'src/global-config';

import { AgentListView } from 'src/sections/agent';

// ----------------------------------------------------------------------

const metadata = { title: `Agents - ${CONFIG.appName}` };

export default function Page() {
  return (
    <>
      <Helmet>
        <title>{metadata.title}</title>
      </Helmet>

      <AgentListView />
    </>
  );
}
