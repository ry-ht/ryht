import { Helmet } from 'react-helmet-async';

import { CONFIG } from 'src/global-config';

import { WorkspaceDetailView } from 'src/sections/cortex/workspace-detail-view';

// ----------------------------------------------------------------------

const metadata = { title: `Workspace Details | Cortex - ${CONFIG.appName}` };

export default function Page() {
  return (
    <>
      <Helmet>
        <title> {metadata.title}</title>
      </Helmet>

      <WorkspaceDetailView />
    </>
  );
}
