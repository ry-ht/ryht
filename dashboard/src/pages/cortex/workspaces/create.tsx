import { Helmet } from 'react-helmet-async';
import { CONFIG } from 'src/global-config';
import { WorkspaceCreateView } from 'src/sections/cortex/workspace-create-view';

export default function Page() {
  return (
    <>
      <Helmet>
        <title>{`Create Workspace - ${CONFIG.appName}`}</title>
      </Helmet>

      <WorkspaceCreateView />
    </>
  );
}
