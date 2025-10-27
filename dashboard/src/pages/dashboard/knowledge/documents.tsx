import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { KnowledgeDocumentsView } from 'src/sections/knowledge/knowledge-documents-view';

// ----------------------------------------------------------------------

const metadata = { title: `Knowledge Documents - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <KnowledgeDocumentsView />;
}
