import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { DocumentCreateView } from 'src/sections/cortex/document-create-view';

export default function Page() {
  useDocumentTitle(`Create Document - ${CONFIG.appName}`);

  return <DocumentCreateView />;
}
