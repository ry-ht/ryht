import { useNavigate } from 'react-router';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Divider from '@mui/material/Divider';
import CardHeader from '@mui/material/CardHeader';
import Typography from '@mui/material/Typography';
import { alpha, useTheme } from '@mui/material/styles';

import { Iconify } from 'src/components/iconify';

// ----------------------------------------------------------------------

export function QuickActionsPanel() {
  const theme = useTheme();
  const navigate = useNavigate();

  const actions: Array<{
    title: string;
    description: string;
    icon: string;
    color: 'primary' | 'secondary' | 'info' | 'success' | 'warning' | 'error';
    href: string;
  }> = [
    {
      title: 'Create Agent',
      description: 'Deploy a new agent instance',
      icon: 'solar:user-plus-rounded-bold',
      color: 'info',
      href: '/dashboard/agents/create',
    },
    {
      title: 'Run Workflow',
      description: 'Execute a workflow definition',
      icon: 'solar:routing-3-bold',
      color: 'primary',
      href: '/dashboard/workflows/create',
    },
    {
      title: 'Create Workspace',
      description: 'Set up a new code workspace',
      icon: 'solar:folder-with-files-bold',
      color: 'success',
      href: '/dashboard/cortex/workspaces/create',
    },
    {
      title: 'Create Document',
      description: 'Add a new documentation page',
      icon: 'solar:document-add-bold',
      color: 'warning',
      href: '/dashboard/cortex/documents/create',
    },
    {
      title: 'Search Memory',
      description: 'Query cognitive memory system',
      icon: 'solar:magnifer-bold',
      color: 'secondary',
      href: '/dashboard/cortex/memory',
    },
    {
      title: 'View Agents',
      description: 'Manage all agent instances',
      icon: 'solar:users-group-rounded-bold',
      color: 'info',
      href: '/dashboard/agents',
    },
    {
      title: 'View Workspaces',
      description: 'Browse code workspaces',
      icon: 'solar:folder-open-bold',
      color: 'success',
      href: '/dashboard/cortex/workspaces',
    },
    {
      title: 'View Documents',
      description: 'Browse documentation',
      icon: 'solar:document-text-bold',
      color: 'warning',
      href: '/dashboard/cortex/documents',
    },
  ];

  return (
    <Card sx={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <CardHeader title="Quick Actions" subheader="Common tasks and shortcuts" />
      <Divider />

      <Box sx={{ flexGrow: 1, overflow: 'auto', p: 2 }}>
        <Stack spacing={1}>
          {actions.map((action) => (
            <QuickActionButton
              key={action.title}
              {...action}
              onClick={() => navigate(action.href)}
            />
          ))}
        </Stack>
      </Box>
    </Card>
  );
}

// ----------------------------------------------------------------------

interface QuickActionButtonProps {
  title: string;
  description: string;
  icon: string;
  color: 'primary' | 'secondary' | 'info' | 'success' | 'warning' | 'error';
  onClick: () => void;
}

function QuickActionButton({
  title,
  description,
  icon,
  color,
  onClick,
}: QuickActionButtonProps) {
  const theme = useTheme();

  return (
    <Button
      fullWidth
      onClick={onClick}
      sx={{
        p: 2,
        height: 'auto',
        justifyContent: 'flex-start',
        textAlign: 'left',
        borderRadius: 1.5,
        border: `1px solid ${theme.palette.divider}`,
        bgcolor: 'transparent',
        '&:hover': {
          bgcolor: alpha(theme.palette[color].main, 0.08),
          borderColor: theme.palette[color].main,
        },
      }}
    >
      <Stack direction="row" spacing={2} alignItems="center" sx={{ width: '100%' }}>
        <Box
          sx={{
            width: 40,
            height: 40,
            flexShrink: 0,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            borderRadius: 1,
            bgcolor: alpha(theme.palette[color].main, 0.16),
          }}
        >
          <Iconify icon={icon} width={24} sx={{ color: theme.palette[color].main }} />
        </Box>

        <Box sx={{ flexGrow: 1, minWidth: 0 }}>
          <Typography variant="subtitle2" noWrap>
            {title}
          </Typography>
          <Typography variant="caption" color="text.secondary" noWrap>
            {description}
          </Typography>
        </Box>

        <Iconify
          icon="eva:arrow-ios-forward-fill"
          width={20}
          sx={{ color: 'text.disabled', flexShrink: 0 }}
        />
      </Stack>
    </Button>
  );
}
