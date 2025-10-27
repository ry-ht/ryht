import { Helmet } from 'react-helmet-async';
import { CONFIG } from 'src/global-config';
import { WorkspaceListView } from 'src/sections/cortex';

export default function Page() {
  return (
    <>
      <Helmet>
        <title>{`Workspaces - ${CONFIG.appName}`}</title>
      </Helmet>

      <WorkspaceListView />
    </>
  );
}
