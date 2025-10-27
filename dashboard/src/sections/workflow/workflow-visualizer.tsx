import type { WorkflowTask } from 'src/types/axon';

import { useEffect, useRef } from 'react';
import mermaid from 'mermaid';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Typography from '@mui/material/Typography';
import { useTheme } from '@mui/material/styles';

import { getWorkflowStatusColor } from 'src/utils/status-colors';

// ----------------------------------------------------------------------

type WorkflowVisualizerProps = {
  tasks: WorkflowTask[];
  onTaskClick?: (taskId: string) => void;
};

export function WorkflowVisualizer({ tasks, onTaskClick }: WorkflowVisualizerProps) {
  const theme = useTheme();
  const mermaidRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    mermaid.initialize({
      startOnLoad: true,
      theme: theme.palette.mode === 'dark' ? 'dark' : 'default',
      securityLevel: 'loose',
      flowchart: {
        useMaxWidth: true,
        htmlLabels: true,
        curve: 'basis',
      },
    });
  }, [theme.palette.mode]);

  useEffect(() => {
    if (!mermaidRef.current || tasks.length === 0) return;

    const renderDiagram = async () => {
      try {
        const mermaidCode = generateMermaidCode(tasks);
        const { svg } = await mermaid.render('workflow-diagram', mermaidCode);

        if (mermaidRef.current) {
          mermaidRef.current.innerHTML = svg;

          // Add click handlers to nodes
          if (onTaskClick) {
            const nodes = mermaidRef.current.querySelectorAll('.node');
            nodes.forEach((node) => {
              const taskId = node.id.replace('flowchart-', '').replace('-', '');
              if (taskId) {
                node.addEventListener('click', () => onTaskClick(taskId));
                (node as HTMLElement).style.cursor = 'pointer';
              }
            });
          }
        }
      } catch (error) {
        console.error('Failed to render Mermaid diagram:', error);
      }
    };

    renderDiagram();
  }, [tasks, onTaskClick]);

  if (tasks.length === 0) {
    return (
      <Card sx={{ p: 3, textAlign: 'center' }}>
        <Typography variant="body2" color="text.secondary">
          No tasks to visualize
        </Typography>
      </Card>
    );
  }

  return (
    <Card sx={{ p: 3 }}>
      <Box
        ref={mermaidRef}
        sx={{
          '& svg': {
            maxWidth: '100%',
            height: 'auto',
          },
          '& .node': {
            transition: 'all 0.2s',
            '&:hover': {
              opacity: 0.8,
            },
          },
        }}
      />
    </Card>
  );
}

// ----------------------------------------------------------------------

function generateMermaidCode(tasks: WorkflowTask[]): string {
  const lines: string[] = ['graph TD'];

  // Create task nodes with status styling
  tasks.forEach((task) => {
    const statusColor = getStatusColorForMermaid(task.status);
    const label = `${task.name}<br/>${task.status}`;
    lines.push(`  ${task.id}["${label}"]:::${statusColor}`);
  });

  // Create edges for dependencies
  tasks.forEach((task) => {
    if (task.dependencies && task.dependencies.length > 0) {
      task.dependencies.forEach((depId) => {
        lines.push(`  ${depId} --> ${task.id}`);
      });
    }
  });

  // Define style classes
  lines.push('  classDef pending fill:#919EAB,stroke:#637381,color:#fff');
  lines.push('  classDef running fill:#00B8D9,stroke:#006C9C,color:#fff');
  lines.push('  classDef completed fill:#22C55E,stroke:#118D57,color:#fff');
  lines.push('  classDef failed fill:#FF5630,stroke:#B71D18,color:#fff');
  lines.push('  classDef cancelled fill:#FFAB00,stroke:#B76E00,color:#fff');

  return lines.join('\n');
}

function getStatusColorForMermaid(status: string): string {
  const statusMap: Record<string, string> = {
    Pending: 'pending',
    Running: 'running',
    Completed: 'completed',
    Failed: 'failed',
    Cancelled: 'cancelled',
  };

  return statusMap[status] || 'pending';
}
