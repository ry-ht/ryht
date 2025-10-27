import { CONFIG } from 'src/global-config';
import { useDocumentTitle } from 'src/hooks/use-document-title';
import { MemoryConsolidationView } from 'src/sections/cortex';

export default function Page() {
  useDocumentTitle(`Memory Consolidation - ${CONFIG.appName}`);

  return <MemoryConsolidationView />;
}
