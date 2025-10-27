import { Helmet } from 'react-helmet-async';

import { CONFIG } from 'src/global-config';

import { TaskDetailView } from 'src/sections/task/task-detail-view';

// ----------------------------------------------------------------------

const metadata = { title: `Task Details | Dashboard - ${CONFIG.appName}` };

export default function Page() {
  return (
    <>
      <Helmet>
        <title> {metadata.title}</title>
      </Helmet>

      <TaskDetailView />
    </>
  );
}
