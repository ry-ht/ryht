import { Helmet } from 'react-helmet-async';

import { CONFIG } from 'src/global-config';

import { CodeUnitsView } from 'src/sections/cortex/code-units-view';

// ----------------------------------------------------------------------

const metadata = { title: `Code Units | Cortex - ${CONFIG.appName}` };

export default function Page() {
  return (
    <>
      <Helmet>
        <title> {metadata.title}</title>
      </Helmet>

      <CodeUnitsView />
    </>
  );
}
