import { Helmet } from 'react-helmet-async';
import { CONFIG } from 'src/global-config';
import { DocumentCreateView } from 'src/sections/cortex/document-create-view';

export default function Page() {
  return (
    <>
      <Helmet>
        <title>{`Create Document - ${CONFIG.appName}`}</title>
      </Helmet>

      <DocumentCreateView />
    </>
  );
}
