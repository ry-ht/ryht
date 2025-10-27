import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { CoordinationLocksView } from 'src/sections/coordination/coordination-locks-view';

// ----------------------------------------------------------------------

const metadata = { title: `Distributed Locks - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <CoordinationLocksView />;
}
