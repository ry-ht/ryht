import type { TelemetryData, TelemetryEndpoint } from 'src/types/axon';

import useSWR from 'swr';
import { useState } from 'react';
import { Bar, Line, Cell, XAxis, YAxis, Legend, Tooltip, BarChart, LineChart, CartesianGrid, ResponsiveContainer } from 'recharts';

import Grid from '@mui/material/Grid';
import Card from '@mui/material/Card';
import Table from '@mui/material/Table';
import Stack from '@mui/material/Stack';
import Select from '@mui/material/Select';
import TableRow from '@mui/material/TableRow';
import MenuItem from '@mui/material/MenuItem';
import TableBody from '@mui/material/TableBody';
import TableCell from '@mui/material/TableCell';
import TableHead from '@mui/material/TableHead';
import Typography from '@mui/material/Typography';
import CardHeader from '@mui/material/CardHeader';
import InputLabel from '@mui/material/InputLabel';
import CardContent from '@mui/material/CardContent';
import FormControl from '@mui/material/FormControl';
import TableContainer from '@mui/material/TableContainer';

import { axonFetcher, axonEndpoints } from 'src/lib/axon-client';

import { Iconify } from 'src/components/iconify';
import { AnimateCountUp } from 'src/components/animate';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

// ----------------------------------------------------------------------

const COLORS = ['#1976d2', '#4caf50', '#ff9800', '#f44336', '#9c27b0'];

// ----------------------------------------------------------------------

export function TelemetryView() {
  const [timeRange, setTimeRange] = useState<number>(3600);

  // Fetch telemetry data
  const { data: telemetry } = useSWR<TelemetryData>(
    axonEndpoints.telemetry,
    axonFetcher,
    { refreshInterval: 5000 }
  );

  // Mock endpoint data
  const mockEndpoints: TelemetryEndpoint[] = [
    {
      endpoint: '/agents',
      method: 'GET',
      total_requests: 1245,
      success_rate: 99.2,
      avg_response_time_ms: 45,
      p50_ms: 38,
      p95_ms: 85,
      p99_ms: 120,
      error_count: 10,
    },
    {
      endpoint: '/agents/:id',
      method: 'GET',
      total_requests: 856,
      success_rate: 98.5,
      avg_response_time_ms: 32,
      p50_ms: 28,
      p95_ms: 65,
      p99_ms: 95,
      error_count: 13,
    },
    {
      endpoint: '/workflows',
      method: 'POST',
      total_requests: 432,
      success_rate: 96.8,
      avg_response_time_ms: 125,
      p50_ms: 110,
      p95_ms: 245,
      p99_ms: 380,
      error_count: 14,
    },
    {
      endpoint: '/metrics',
      method: 'GET',
      total_requests: 2341,
      success_rate: 99.8,
      avg_response_time_ms: 18,
      p50_ms: 15,
      p95_ms: 28,
      p99_ms: 45,
      error_count: 5,
    },
  ];

  // Mock request rate data
  const mockRequestData = [
    { time: '00:00', requests: 120, errors: 2 },
    { time: '01:00', requests: 95, errors: 1 },
    { time: '02:00', requests: 78, errors: 0 },
    { time: '03:00', requests: 145, errors: 3 },
    { time: '04:00', requests: 210, errors: 5 },
    { time: '05:00', requests: 189, errors: 2 },
  ];

  const totalRequests = telemetry?.total_requests || 0;
  const successRate = telemetry?.successful_requests && telemetry?.total_requests
    ? (telemetry.successful_requests / telemetry.total_requests) * 100
    : 0;
  const errorRate = telemetry?.error_rate || 0;
  const avgResponseTime = telemetry?.avg_response_time_ms || 0;

  return (
    <Stack spacing={2}>
      <CustomBreadcrumbs
        heading="Telemetry"
        links={[
          { name: 'Dashboard', href: '/' },
          { name: 'Telemetry' },
        ]}
        action={
          <FormControl sx={{ minWidth: 200 }}>
            <InputLabel>Time Range</InputLabel>
            <Select
              value={timeRange}
              label="Time Range"
              onChange={(e) => setTimeRange(e.target.value as number)}
            >
              <MenuItem value={300}>Last 5 minutes</MenuItem>
              <MenuItem value={900}>Last 15 minutes</MenuItem>
              <MenuItem value={3600}>Last hour</MenuItem>
              <MenuItem value={86400}>Last 24 hours</MenuItem>
            </Select>
          </FormControl>
        }
        sx={{ mb: 3 }}
      />

      <Grid container spacing={3}>
        {/* Summary Cards */}
        <Grid size={{ xs: 12, sm: 6, md: 3 }}>
          <Card sx={{ p: 3 }}>
            <Stack spacing={1}>
              <Stack direction="row" alignItems="center" spacing={1}>
                <Iconify icon="solar:chart-2-bold" width={24} />
                <Typography variant="subtitle2">Total Requests</Typography>
              </Stack>
              <Typography variant="h3">
                <AnimateCountUp to={totalRequests} />
              </Typography>
              <Typography variant="caption" color="text.secondary">
                Last {timeRange / 60} minutes
              </Typography>
            </Stack>
          </Card>
        </Grid>

        <Grid size={{ xs: 12, sm: 6, md: 3 }}>
          <Card sx={{ p: 3 }}>
            <Stack spacing={1}>
              <Stack direction="row" alignItems="center" spacing={1}>
                <Iconify icon="solar:check-circle-bold" width={24} />
                <Typography variant="subtitle2">Success Rate</Typography>
              </Stack>
              <Typography variant="h3" color="success.main">
                {successRate.toFixed(1)}%
              </Typography>
              <Typography variant="caption" color="text.secondary">
                Request success rate
              </Typography>
            </Stack>
          </Card>
        </Grid>

        <Grid size={{ xs: 12, sm: 6, md: 3 }}>
          <Card sx={{ p: 3 }}>
            <Stack spacing={1}>
              <Stack direction="row" alignItems="center" spacing={1}>
                <Iconify icon="solar:danger-triangle-bold" width={24} />
                <Typography variant="subtitle2">Error Rate</Typography>
              </Stack>
              <Typography variant="h3" color="error.main">
                {errorRate.toFixed(2)}%
              </Typography>
              <Typography variant="caption" color="text.secondary">
                Request error rate
              </Typography>
            </Stack>
          </Card>
        </Grid>

        <Grid size={{ xs: 12, sm: 6, md: 3 }}>
          <Card sx={{ p: 3 }}>
            <Stack spacing={1}>
              <Stack direction="row" alignItems="center" spacing={1}>
                <Iconify icon="solar:clock-circle-bold" width={24} />
                <Typography variant="subtitle2">Avg Latency</Typography>
              </Stack>
              <Typography variant="h3">
                {avgResponseTime.toFixed(0)}ms
              </Typography>
              <Typography variant="caption" color="text.secondary">
                Average response time
              </Typography>
            </Stack>
          </Card>
        </Grid>

        {/* Request Rate Chart */}
        <Grid size={{ xs: 12, lg: 8 }}>
          <Card>
            <CardHeader title="Request & Error Trends" />
            <CardContent>
              <ResponsiveContainer width="100%" height={300}>
                <LineChart data={mockRequestData}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="time" fontSize={12} />
                  <YAxis fontSize={12} />
                  <Tooltip contentStyle={{ fontSize: 12 }} />
                  <Legend wrapperStyle={{ fontSize: 12 }} />
                  <Line type="monotone" dataKey="requests" stroke="#1976d2" strokeWidth={2} name="Requests" />
                  <Line type="monotone" dataKey="errors" stroke="#f44336" strokeWidth={2} name="Errors" />
                </LineChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>
        </Grid>

        {/* Response Time Distribution */}
        <Grid size={{ xs: 12, lg: 4 }}>
          <Card>
            <CardHeader title="Response Time Distribution" />
            <CardContent>
              <ResponsiveContainer width="100%" height={300}>
                <BarChart data={mockEndpoints.map((e) => ({ name: e.endpoint, avg: e.avg_response_time_ms }))}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="name" fontSize={10} angle={-45} textAnchor="end" height={80} />
                  <YAxis fontSize={12} />
                  <Tooltip contentStyle={{ fontSize: 12 }} />
                  <Bar dataKey="avg" fill="#1976d2" name="Avg (ms)">
                    {mockEndpoints.map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
                    ))}
                  </Bar>
                </BarChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>
        </Grid>

        {/* Endpoint Performance Table */}
        <Grid size={{ xs: 12 }}>
          <Card>
            <CardHeader title="Endpoint Performance" />
            <TableContainer>
              <Table>
                <TableHead>
                  <TableRow>
                    <TableCell>Endpoint</TableCell>
                    <TableCell>Method</TableCell>
                    <TableCell align="right">Total Requests</TableCell>
                    <TableCell align="right">Success Rate</TableCell>
                    <TableCell align="right">Avg Response</TableCell>
                    <TableCell align="right">P50</TableCell>
                    <TableCell align="right">P95</TableCell>
                    <TableCell align="right">P99</TableCell>
                    <TableCell align="right">Errors</TableCell>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {mockEndpoints.map((endpoint) => (
                    <TableRow key={endpoint.endpoint} hover>
                      <TableCell>
                        <Typography variant="body2" fontFamily="monospace">
                          {endpoint.endpoint}
                        </Typography>
                      </TableCell>
                      <TableCell>
                        <Typography
                          variant="caption"
                          sx={{
                            px: 1,
                            py: 0.5,
                            borderRadius: 1,
                            bgcolor: endpoint.method === 'GET' ? 'info.lighter' : 'warning.lighter',
                            color: endpoint.method === 'GET' ? 'info.darker' : 'warning.darker',
                          }}
                        >
                          {endpoint.method}
                        </Typography>
                      </TableCell>
                      <TableCell align="right">{endpoint.total_requests.toLocaleString()}</TableCell>
                      <TableCell align="right">
                        <Typography
                          variant="body2"
                          sx={{ color: endpoint.success_rate >= 99 ? 'success.main' : endpoint.success_rate >= 95 ? 'warning.main' : 'error.main' }}
                        >
                          {endpoint.success_rate.toFixed(1)}%
                        </Typography>
                      </TableCell>
                      <TableCell align="right">{endpoint.avg_response_time_ms}ms</TableCell>
                      <TableCell align="right">{endpoint.p50_ms}ms</TableCell>
                      <TableCell align="right">{endpoint.p95_ms}ms</TableCell>
                      <TableCell align="right">{endpoint.p99_ms}ms</TableCell>
                      <TableCell align="right">
                        <Typography variant="body2" color={endpoint.error_count > 10 ? 'error.main' : 'text.secondary'}>
                          {endpoint.error_count}
                        </Typography>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </TableContainer>
          </Card>
        </Grid>
      </Grid>
    </Stack>
  );
}
