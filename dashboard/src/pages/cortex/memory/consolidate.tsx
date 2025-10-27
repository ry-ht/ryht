import { Helmet } from 'react-helmet-async';
import { CONFIG } from 'src/global-config';
import { MemoryConsolidationView } from 'src/sections/cortex';

export default function Page() {
  return (
    <>
      <Helmet>
        <title>{`Memory Consolidation - ${CONFIG.appName}`}</title>
      </Helmet>

      <MemoryConsolidationView />
    </>
  );
}
