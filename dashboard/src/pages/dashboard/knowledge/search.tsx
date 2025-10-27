import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { KnowledgeSearchView } from 'src/sections/knowledge/knowledge-search-view';

// ----------------------------------------------------------------------

const metadata = { title: `Semantic Search - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <KnowledgeSearchView />;
}
