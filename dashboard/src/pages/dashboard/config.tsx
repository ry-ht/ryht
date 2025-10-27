import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { ConfigView } from 'src/sections/config/config-view';

// ----------------------------------------------------------------------

const metadata = { title: `System Configuration - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <ConfigView />;
}
