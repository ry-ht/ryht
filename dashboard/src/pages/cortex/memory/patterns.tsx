import { Helmet } from 'react-helmet-async';
import { CONFIG } from 'src/global-config';
import { MemoryPatternsView } from 'src/sections/cortex';

export default function Page() {
  return (
    <>
      <Helmet>
        <title>{`Learned Patterns - ${CONFIG.appName}`}</title>
      </Helmet>

      <MemoryPatternsView />
    </>
  );
}
