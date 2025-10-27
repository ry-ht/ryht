import { CONFIG } from 'src/global-config';
import { useDocumentTitle } from 'src/hooks/use-document-title';
import { DocumentView } from 'src/sections/cortex';

export default function Page() {
  useDocumentTitle(`Document - ${CONFIG.appName}`);

  return <DocumentView />;
}
