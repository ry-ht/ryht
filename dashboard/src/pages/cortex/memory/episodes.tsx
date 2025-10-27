import { CONFIG } from 'src/global-config';
import { useDocumentTitle } from 'src/hooks/use-document-title';
import { MemoryEpisodesView } from 'src/sections/cortex';

export default function Page() {
  useDocumentTitle(`Memory Episodes - ${CONFIG.appName}`);

  return <MemoryEpisodesView />;
}
