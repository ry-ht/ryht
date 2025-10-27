import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { AgentDetailView } from 'src/sections/agent/agent-detail-view';

// ----------------------------------------------------------------------

const metadata = { title: `Agent Details | Dashboard - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <AgentDetailView />;
}
