import Box from '@mui/material/Box';
import { alpha, useTheme } from '@mui/material/styles';

// ----------------------------------------------------------------------

interface StatusIndicatorProps {
  status: 'healthy' | 'degraded' | 'unhealthy';
  size?: number;
}

export function StatusIndicator({ status, size = 8 }: StatusIndicatorProps) {
  const theme = useTheme();

  const getColor = () => {
    switch (status) {
      case 'healthy':
        return theme.palette.success.main;
      case 'degraded':
        return theme.palette.warning.main;
      case 'unhealthy':
        return theme.palette.error.main;
      default:
        return theme.palette.grey[500];
    }
  };

  const color = getColor();

  return (
    <Box
      sx={{
        width: size * 2,
        height: size * 2,
        borderRadius: '50%',
        bgcolor: alpha(color, 0.16),
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
      }}
    >
      <Box
        sx={{
          width: size,
          height: size,
          borderRadius: '50%',
          bgcolor: color,
          animation: status === 'healthy' ? 'pulse 2s infinite' : 'none',
          '@keyframes pulse': {
            '0%': {
              boxShadow: `0 0 0 0 ${alpha(color, 0.7)}`,
            },
            '70%': {
              boxShadow: `0 0 0 ${size}px ${alpha(color, 0)}`,
            },
            '100%': {
              boxShadow: `0 0 0 0 ${alpha(color, 0)}`,
            },
          },
        }}
      />
    </Box>
  );
}
