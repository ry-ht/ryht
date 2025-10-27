import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { MemoryEpisodesView } from 'src/sections/cortex';

export default function Page() {
  useDocumentTitle(`Memory Episodes - ${CONFIG.appName}`);

  return <MemoryEpisodesView />;
}
