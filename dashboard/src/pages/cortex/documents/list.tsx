import { Helmet } from 'react-helmet-async';
import { CONFIG } from 'src/global-config';
import { DocumentListView } from 'src/sections/cortex';

export default function Page() {
  return (
    <>
      <Helmet>
        <title>{`Documents - ${CONFIG.appName}`}</title>
      </Helmet>

      <DocumentListView />
    </>
  );
}
