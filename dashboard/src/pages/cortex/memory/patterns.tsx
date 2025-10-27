import { CONFIG } from 'src/global-config';
import { useDocumentTitle } from 'src/hooks/use-document-title';
import { MemoryPatternsView } from 'src/sections/cortex';

export default function Page() {
  useDocumentTitle(`Learned Patterns - ${CONFIG.appName}`);

  return <MemoryPatternsView />;
}
