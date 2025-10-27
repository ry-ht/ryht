import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { AgentTypeView } from 'src/sections/agent/agent-type-view';

// ----------------------------------------------------------------------

const metadata = { title: `Reviewer Agents - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <AgentTypeView agentType="Reviewer" />;
}
