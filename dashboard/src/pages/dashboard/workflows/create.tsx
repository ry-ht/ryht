import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { WorkflowCreateView } from 'src/sections/workflow';

// ----------------------------------------------------------------------

const metadata = { title: `Run Workflow - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <WorkflowCreateView />;
}
