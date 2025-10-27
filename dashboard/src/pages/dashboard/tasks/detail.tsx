import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { TaskDetailView } from 'src/sections/task/task-detail-view';

// ----------------------------------------------------------------------

const metadata = { title: `Task Details | Dashboard - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <TaskDetailView />;
}
