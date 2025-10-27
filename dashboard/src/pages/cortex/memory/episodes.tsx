import { Helmet } from 'react-helmet-async';
import { CONFIG } from 'src/global-config';
import { MemoryEpisodesView } from 'src/sections/cortex';

export default function Page() {
  return (
    <>
      <Helmet>
        <title>{`Memory Episodes - ${CONFIG.appName}`}</title>
      </Helmet>

      <MemoryEpisodesView />
    </>
  );
}
