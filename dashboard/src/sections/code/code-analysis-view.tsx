import { useState } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Grid from '@mui/material/Grid';
import Stack from '@mui/material/Stack';
import Select from '@mui/material/Select';
import MenuItem from '@mui/material/MenuItem';
import Typography from '@mui/material/Typography';
import InputLabel from '@mui/material/InputLabel';
import FormControl from '@mui/material/FormControl';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

const MOCK_ANALYSIS = {
  totalFiles: 342,
  totalLines: 45678,
  codeLines: 38456,
  commentLines: 5123,
  blankLines: 2099,
  languages: [
    { name: 'TypeScript', files: 245, lines: 32456, percentage: 71 },
    { name: 'JavaScript', files: 67, lines: 8934, percentage: 20 },
    { name: 'CSS', files: 23, lines: 3456, percentage: 8 },
    { name: 'JSON', files: 7, lines: 832, percentage: 1 },
  ],
  complexity: {
    average: 8.5,
    max: 45,
    high: 12,
    medium: 89,
    low: 241,
  },
};

export function CodeAnalysisView() {
  const [selectedWorkspace, setSelectedWorkspace] = useState('default');

  return (
    <>
      <CustomBreadcrumbs
        heading="Code Analysis"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Code' },
          { name: 'Analysis' },
        ]}
        sx={{ mb: 3 }}
      />

      <Stack spacing={2}>
        <Card sx={{ p: 3 }}>
          <Stack direction="row" justifyContent="space-between" alignItems="center">
            <Box>
              <Typography variant="h6" sx={{ mb: 0.5 }}>
                Codebase Analysis Dashboard
              </Typography>
              <Typography variant="body2" color="text.secondary">
                Comprehensive analysis of code structure, complexity, and metrics.
              </Typography>
            </Box>
            <FormControl sx={{ minWidth: 200 }}>
              <InputLabel>Workspace</InputLabel>
              <Select
                value={selectedWorkspace}
                label="Workspace"
                onChange={(e) => setSelectedWorkspace(e.target.value)}
              >
                <MenuItem value="default">Default Workspace</MenuItem>
                <MenuItem value="project-1">Project 1</MenuItem>
                <MenuItem value="project-2">Project 2</MenuItem>
              </Select>
            </FormControl>
          </Stack>
        </Card>

        <Grid container spacing={2}>
          <Grid size={{ xs: 6, md: 3 }}>
            <Card sx={{ p: 2.5 }}>
              <Stack spacing={1}>
                <Iconify icon="mdi:file-code" width={32} color="primary.main" />
                <Typography variant="h4">{MOCK_ANALYSIS.totalFiles}</Typography>
                <Typography variant="body2" color="text.secondary">
                  Total Files
                </Typography>
              </Stack>
            </Card>
          </Grid>
          <Grid size={{ xs: 6, md: 3 }}>
            <Card sx={{ p: 2.5 }}>
              <Stack spacing={1}>
                <Iconify icon="mdi:format-line-spacing" width={32} color="info.main" />
                <Typography variant="h4">{MOCK_ANALYSIS.totalLines.toLocaleString()}</Typography>
                <Typography variant="body2" color="text.secondary">
                  Total Lines
                </Typography>
              </Stack>
            </Card>
          </Grid>
          <Grid size={{ xs: 6, md: 3 }}>
            <Card sx={{ p: 2.5 }}>
              <Stack spacing={1}>
                <Iconify icon="mdi:code-braces" width={32} color="success.main" />
                <Typography variant="h4">{MOCK_ANALYSIS.codeLines.toLocaleString()}</Typography>
                <Typography variant="body2" color="text.secondary">
                  Code Lines
                </Typography>
              </Stack>
            </Card>
          </Grid>
          <Grid size={{ xs: 6, md: 3 }}>
            <Card sx={{ p: 2.5 }}>
              <Stack spacing={1}>
                <Iconify icon="mdi:comment-text" width={32} color="warning.main" />
                <Typography variant="h4">{MOCK_ANALYSIS.commentLines.toLocaleString()}</Typography>
                <Typography variant="body2" color="text.secondary">
                  Comment Lines
                </Typography>
              </Stack>
            </Card>
          </Grid>
        </Grid>

        <Grid container spacing={2}>
          <Grid size={{ xs: 12, md: 6 }}>
            <Card sx={{ p: 3 }}>
              <Typography variant="h6" sx={{ mb: 2 }}>
                Language Distribution
              </Typography>
              <Stack spacing={2}>
                {MOCK_ANALYSIS.languages.map((lang) => (
                  <Box key={lang.name}>
                    <Stack direction="row" justifyContent="space-between" sx={{ mb: 0.5 }}>
                      <Typography variant="body2">{lang.name}</Typography>
                      <Typography variant="body2" color="text.secondary">
                        {lang.files} files • {lang.lines.toLocaleString()} lines
                      </Typography>
                    </Stack>
                    <Box sx={{ position: 'relative' }}>
                      <Box
                        sx={{
                          height: 8,
                          borderRadius: 1,
                          bgcolor: 'background.neutral',
                        }}
                      >
                        <Box
                          sx={{
                            height: '100%',
                            width: `${lang.percentage}%`,
                            borderRadius: 1,
                            bgcolor: 'primary.main',
                          }}
                        />
                      </Box>
                    </Box>
                  </Box>
                ))}
              </Stack>
            </Card>
          </Grid>

          <Grid size={{ xs: 12, md: 6 }}>
            <Card sx={{ p: 3 }}>
              <Typography variant="h6" sx={{ mb: 2 }}>
                Code Complexity
              </Typography>
              <Stack spacing={2}>
                <Stack direction="row" justifyContent="space-between">
                  <Typography variant="body2" color="text.secondary">
                    Average Complexity
                  </Typography>
                  <Typography variant="h6">{MOCK_ANALYSIS.complexity.average}</Typography>
                </Stack>
                <Stack direction="row" justifyContent="space-between">
                  <Typography variant="body2" color="text.secondary">
                    Maximum Complexity
                  </Typography>
                  <Typography variant="h6" color="error.main">
                    {MOCK_ANALYSIS.complexity.max}
                  </Typography>
                </Stack>
                <Box sx={{ pt: 1 }}>
                  <Stack direction="row" spacing={2} justifyContent="space-between">
                    <Stack spacing={0.5} flex={1} alignItems="center">
                      <Label color="error" variant="soft">
                        High
                      </Label>
                      <Typography variant="h5">{MOCK_ANALYSIS.complexity.high}</Typography>
                    </Stack>
                    <Stack spacing={0.5} flex={1} alignItems="center">
                      <Label color="warning" variant="soft">
                        Medium
                      </Label>
                      <Typography variant="h5">{MOCK_ANALYSIS.complexity.medium}</Typography>
                    </Stack>
                    <Stack spacing={0.5} flex={1} alignItems="center">
                      <Label color="success" variant="soft">
                        Low
                      </Label>
                      <Typography variant="h5">{MOCK_ANALYSIS.complexity.low}</Typography>
                    </Stack>
                  </Stack>
                </Box>
              </Stack>
            </Card>
          </Grid>
        </Grid>
      </Stack>
    </>
  );
}
