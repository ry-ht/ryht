import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { MemoryPatternsView } from 'src/sections/memory/memory-patterns-view';

// ----------------------------------------------------------------------

const metadata = { title: `Learned Patterns - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <MemoryPatternsView />;
}
