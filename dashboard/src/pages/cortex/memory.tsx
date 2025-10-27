import { Helmet } from 'react-helmet-async';
import { CONFIG } from 'src/global-config';
import { MemorySearchView } from 'src/sections/cortex';

export default function Page() {
  return (
    <>
      <Helmet>
        <title>{`Memory Search - ${CONFIG.appName}`}</title>
      </Helmet>

      <MemorySearchView />
    </>
  );
}
