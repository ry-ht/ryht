import { Helmet } from 'react-helmet-async';
import { CONFIG } from 'src/global-config';
import { CortexOverview } from 'src/sections/cortex';

export default function Page() {
  return (
    <>
      <Helmet>
        <title>{`Cortex Overview - ${CONFIG.appName}`}</title>
      </Helmet>

      <CortexOverview />
    </>
  );
}
