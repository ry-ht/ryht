import { CONFIG } from 'src/global-config';

import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CodeUnitsView } from 'src/sections/cortex/code-units-view';

// ----------------------------------------------------------------------

const metadata = { title: `Code Units | Cortex - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <CodeUnitsView />;
}
