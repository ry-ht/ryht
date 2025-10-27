import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { MemoryWorkingView } from 'src/sections/memory/memory-working-view';

// ----------------------------------------------------------------------

const metadata = { title: `Working Memory - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <MemoryWorkingView />;
}
