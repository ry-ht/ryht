import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { CodeVfsView } from 'src/sections/code/code-vfs-view';

// ----------------------------------------------------------------------

const metadata = { title: `Virtual File System - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <CodeVfsView />;
}
