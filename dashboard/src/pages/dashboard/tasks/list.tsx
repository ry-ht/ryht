import { Helmet } from 'react-helmet-async';

import { CONFIG } from 'src/global-config';

import { TaskListView } from 'src/sections/task/task-list-view';

// ----------------------------------------------------------------------

const metadata = { title: `Tasks | Dashboard - ${CONFIG.appName}` };

export default function Page() {
  return (
    <>
      <Helmet>
        <title> {metadata.title}</title>
      </Helmet>

      <TaskListView />
    </>
  );
}
