import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { CoordinationSessionsView } from 'src/sections/coordination/coordination-sessions-view';

// ----------------------------------------------------------------------

const metadata = { title: `Coordination Sessions - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <CoordinationSessionsView />;
}
