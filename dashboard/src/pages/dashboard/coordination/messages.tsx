import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { CoordinationMessagesView } from 'src/sections/coordination/coordination-messages-view';

// ----------------------------------------------------------------------

const metadata = { title: `Inter-Agent Messages - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <CoordinationMessagesView />;
}
