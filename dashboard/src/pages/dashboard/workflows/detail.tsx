import { Helmet } from 'react-helmet-async';

import { CONFIG } from 'src/global-config';

import { WorkflowDetailView } from 'src/sections/workflow/workflow-detail-view';

// ----------------------------------------------------------------------

const metadata = { title: `Workflow Details | Dashboard - ${CONFIG.appName}` };

export default function Page() {
  return (
    <>
      <Helmet>
        <title> {metadata.title}</title>
      </Helmet>

      <WorkflowDetailView />
    </>
  );
}
