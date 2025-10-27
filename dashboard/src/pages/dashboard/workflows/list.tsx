import { CONFIG } from 'src/global-config';

import { useDocumentTitle } from 'src/hooks/use-document-title';

import { WorkflowListView } from 'src/sections/workflow';

// ----------------------------------------------------------------------

const metadata = { title: `Workflows - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <WorkflowListView />;
}
