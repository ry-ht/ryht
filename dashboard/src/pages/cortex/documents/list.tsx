import { CONFIG } from 'src/global-config';
import { useDocumentTitle } from 'src/hooks/use-document-title';
import { DocumentListView } from 'src/sections/cortex';

export default function Page() {
  useDocumentTitle(`Documents - ${CONFIG.appName}`);

  return <DocumentListView />;
}
