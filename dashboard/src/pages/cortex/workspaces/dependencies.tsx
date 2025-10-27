import { CONFIG } from 'src/global-config';

import { useDocumentTitle } from 'src/hooks/use-document-title';

import { DependencyGraphView } from 'src/sections/cortex/dependency-graph-view';

// ----------------------------------------------------------------------

const metadata = { title: `Dependencies | Cortex - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <DependencyGraphView />;
}
