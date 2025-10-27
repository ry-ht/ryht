import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { MemorySearchView } from 'src/sections/cortex';

export default function Page() {
  useDocumentTitle(`Memory Search - ${CONFIG.appName}`);

  return <MemorySearchView />;
}
