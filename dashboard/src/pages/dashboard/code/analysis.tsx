import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { CodeAnalysisView } from 'src/sections/code/code-analysis-view';

// ----------------------------------------------------------------------

const metadata = { title: `Code Analysis - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <CodeAnalysisView />;
}
