import { Link } from 'react-router';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import { alpha, useTheme } from '@mui/material/styles';

import { Iconify } from 'src/components/iconify';
import { AnimateCountUp } from 'src/components/animate';

// ----------------------------------------------------------------------

interface MetricCardProps {
  title: string;
  value: number;
  total?: number;
  icon: string;
  color?: 'primary' | 'secondary' | 'info' | 'success' | 'warning' | 'error';
  href?: string;
}

export function MetricCard({
  title,
  value,
  total,
  icon,
  color = 'primary',
  href,
}: MetricCardProps) {
  const theme = useTheme();

  const iconColor = theme.palette[color].main;
  const bgColor = alpha(theme.palette[color].main, 0.12);

  const content = (
    <Card
      sx={{
        p: 3,
        cursor: href ? 'pointer' : 'default',
        transition: 'all 0.3s ease-in-out',
        '&:hover': href
          ? {
              boxShadow: theme.shadows[8],
              transform: 'translateY(-4px)',
            }
          : {},
      }}
    >
      <Stack spacing={2}>
        <Stack direction="row" alignItems="center" justifyContent="space-between">
          <Box
            sx={{
              width: 56,
              height: 56,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              borderRadius: 2,
              bgcolor: bgColor,
            }}
          >
            <Iconify icon={icon} width={32} sx={{ color: iconColor }} />
          </Box>

          {href && (
            <Iconify
              icon="eva:arrow-ios-forward-fill"
              width={24}
              sx={{ color: 'text.disabled' }}
            />
          )}
        </Stack>

        <Stack spacing={0.5}>
          <Typography variant="h3">
            <AnimateCountUp to={value} />
          </Typography>
          <Typography variant="subtitle2" sx={{ color: 'text.secondary' }}>
            {title}
          </Typography>
          {total !== undefined && (
            <Typography variant="caption" sx={{ color: 'text.disabled' }}>
              {total} total
            </Typography>
          )}
        </Stack>
      </Stack>
    </Card>
  );

  if (href) {
    return (
      <Link to={href} style={{ textDecoration: 'none' }}>
        {content}
      </Link>
    );
  }

  return content;
}
