import { Helmet } from 'react-helmet-async';

import { CONFIG } from 'src/global-config';

import { WorkflowListView } from 'src/sections/workflow';

// ----------------------------------------------------------------------

const metadata = { title: `Workflows - ${CONFIG.appName}` };

export default function Page() {
  return (
    <>
      <Helmet>
        <title>{metadata.title}</title>
      </Helmet>

      <WorkflowListView />
    </>
  );
}
