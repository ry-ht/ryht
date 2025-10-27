import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { CodeQualityView } from 'src/sections/code/code-quality-view';

// ----------------------------------------------------------------------

const metadata = { title: `Code Quality - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <CodeQualityView />;
}
