import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { AgentCreateView } from 'src/sections/agent';

// ----------------------------------------------------------------------

const metadata = { title: `Create Agent - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <AgentCreateView />;
}
