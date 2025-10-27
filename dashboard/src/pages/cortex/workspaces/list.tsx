import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { WorkspaceListView } from 'src/sections/cortex';

export default function Page() {
  useDocumentTitle(`Workspaces - ${CONFIG.appName}`);

  return <WorkspaceListView />;
}
