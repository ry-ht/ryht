import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { VfsBrowserView } from 'src/sections/cortex/vfs-browser-view';

export default function Page() {
  useDocumentTitle(`File Browser - ${CONFIG.appName}`);

  return <VfsBrowserView />;
}
