import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { DocumentView } from 'src/sections/cortex';

export default function Page() {
  useDocumentTitle(`Document - ${CONFIG.appName}`);

  return <DocumentView />;
}
