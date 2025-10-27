import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { MemorySemanticView } from 'src/sections/memory/memory-semantic-view';

// ----------------------------------------------------------------------

const metadata = { title: `Semantic Memory - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <MemorySemanticView />;
}
