import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { CortexOverview } from 'src/sections/cortex';

export default function Page() {
  useDocumentTitle(`Cortex Overview - ${CONFIG.appName}`);

  return <CortexOverview />;
}
