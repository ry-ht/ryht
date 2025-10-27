import { CONFIG } from 'src/global-config';

import { useDocumentTitle } from 'src/hooks/use-document-title';

import { WorkspaceDetailView } from 'src/sections/cortex/workspace-detail-view';

// ----------------------------------------------------------------------

const metadata = { title: `Workspace Details | Cortex - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <WorkspaceDetailView />;
}
