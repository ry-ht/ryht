import { Helmet } from 'react-helmet-async';

import { CONFIG } from 'src/global-config';

import { DependencyGraphView } from 'src/sections/cortex/dependency-graph-view';

// ----------------------------------------------------------------------

const metadata = { title: `Dependencies | Cortex - ${CONFIG.appName}` };

export default function Page() {
  return (
    <>
      <Helmet>
        <title> {metadata.title}</title>
      </Helmet>

      <DependencyGraphView />
    </>
  );
}
