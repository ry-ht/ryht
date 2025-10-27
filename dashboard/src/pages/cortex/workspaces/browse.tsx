import { Helmet } from 'react-helmet-async';
import { CONFIG } from 'src/global-config';
import { VfsBrowserView } from 'src/sections/cortex/vfs-browser-view';

export default function Page() {
  return (
    <>
      <Helmet>
        <title>{`File Browser - ${CONFIG.appName}`}</title>
      </Helmet>

      <VfsBrowserView />
    </>
  );
}
