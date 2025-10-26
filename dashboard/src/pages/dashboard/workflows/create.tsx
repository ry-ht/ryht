import { Helmet } from 'react-helmet-async';

import { CONFIG } from 'src/global-config';

import { WorkflowCreateView } from 'src/sections/workflow';

// ----------------------------------------------------------------------

const metadata = { title: `Run Workflow - ${CONFIG.appName}` };

export default function Page() {
  return (
    <>
      <Helmet>
        <title>{metadata.title}</title>
      </Helmet>

      <WorkflowCreateView />
    </>
  );
}
