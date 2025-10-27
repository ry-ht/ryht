import { Helmet } from 'react-helmet-async';
import { CONFIG } from 'src/global-config';
import { DocumentView } from 'src/sections/cortex';

export default function Page() {
  return (
    <>
      <Helmet>
        <title>{`Document - ${CONFIG.appName}`}</title>
      </Helmet>

      <DocumentView />
    </>
  );
}
