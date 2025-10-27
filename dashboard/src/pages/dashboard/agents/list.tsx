import { CONFIG } from 'src/global-config';

import { useDocumentTitle } from 'src/hooks/use-document-title';

import { AgentListView } from 'src/sections/agent';

// ----------------------------------------------------------------------

const metadata = { title: `Agents - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <AgentListView />;
}
