import { CONFIG } from 'src/global-config';
import { useDocumentTitle } from 'src/hooks/use-document-title';
import { MemorySearchView } from 'src/sections/cortex';

export default function Page() {
  useDocumentTitle(`Memory Search - ${CONFIG.appName}`);

  return <MemorySearchView />;
}
