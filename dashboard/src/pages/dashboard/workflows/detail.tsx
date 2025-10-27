import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { WorkflowDetailView } from 'src/sections/workflow/workflow-detail-view';

// ----------------------------------------------------------------------

const metadata = { title: `Workflow Details | Dashboard - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <WorkflowDetailView />;
}
