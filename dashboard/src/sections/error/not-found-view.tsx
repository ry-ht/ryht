import { m } from 'framer-motion';

import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Container from '@mui/material/Container';
import Typography from '@mui/material/Typography';

import { RouterLink } from 'src/routes/components';

import { DashboardLayout } from 'src/layouts/dashboard';
import { PageNotFoundIllustration } from 'src/assets/illustrations';

import { varBounce, MotionContainer } from 'src/components/animate';

// ----------------------------------------------------------------------

export function NotFoundView() {
  return (
    <DashboardLayout>
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          minHeight: '100%',
          textAlign: 'center',
        }}
      >
        <Container component={MotionContainer} sx={{ maxWidth: 448 }}>
          <m.div variants={varBounce('in')}>
            <Typography variant="h3" sx={{ mb: 2 }}>
              Sorry, page not found!
            </Typography>
          </m.div>

          <m.div variants={varBounce('in')}>
            <Typography sx={{ color: 'text.secondary' }}>
              Sorry, we couldn&apos;t find the page you&apos;re looking for. Perhaps you&apos;ve mistyped the URL? Be
              sure to check your spelling.
            </Typography>
          </m.div>

          <m.div variants={varBounce('in')}>
            <PageNotFoundIllustration sx={{ my: { xs: 5, sm: 10 } }} />
          </m.div>

          <Button component={RouterLink} href="/" size="large" variant="contained">
            Go to home
          </Button>
        </Container>
      </Box>
    </DashboardLayout>
  );
}
