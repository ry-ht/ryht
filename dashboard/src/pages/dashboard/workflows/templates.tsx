import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { WorkflowTemplatesView } from 'src/sections/workflow/workflow-templates-view';

// ----------------------------------------------------------------------

const metadata = { title: `Workflow Templates - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <WorkflowTemplatesView />;
}
