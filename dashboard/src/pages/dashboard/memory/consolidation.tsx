import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { MemoryConsolidationView } from 'src/sections/memory/memory-consolidation-view';

// ----------------------------------------------------------------------

const metadata = { title: `Memory Consolidation - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <MemoryConsolidationView />;
}
