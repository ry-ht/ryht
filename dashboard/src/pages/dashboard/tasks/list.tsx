import { useDocumentTitle } from 'src/hooks/use-document-title';

import { CONFIG } from 'src/global-config';

import { TaskListView } from 'src/sections/task/task-list-view';

// ----------------------------------------------------------------------

const metadata = { title: `Tasks | Dashboard - ${CONFIG.appName}` };

export default function Page() {
  useDocumentTitle(metadata.title);

  return <TaskListView />;
}
