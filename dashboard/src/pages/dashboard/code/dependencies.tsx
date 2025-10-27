import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { CodeDependenciesView } from 'src/sections/code/code-dependencies-view';

// ----------------------------------------------------------------------

const metadata = { title: `Dependencies - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <CodeDependenciesView />;
}
