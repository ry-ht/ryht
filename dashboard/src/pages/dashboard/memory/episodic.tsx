import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { MemoryEpisodicView } from 'src/sections/memory/memory-episodic-view';

// ----------------------------------------------------------------------

const metadata = { title: `Episodic Memory - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <MemoryEpisodicView />;
}
