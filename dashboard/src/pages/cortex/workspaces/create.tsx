import { CONFIG } from 'src/global-config';
import { useDocumentTitle } from 'src/hooks/use-document-title';
import { WorkspaceCreateView } from 'src/sections/cortex/workspace-create-view';

export default function Page() {
  useDocumentTitle(`Create Workspace - ${CONFIG.appName}`);

  return <WorkspaceCreateView />;
}
