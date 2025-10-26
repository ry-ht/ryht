import type { ReactNode } from 'react';
import type { AlertColor } from '@mui/material/Alert';

import { useState, useCallback, createContext } from 'react';

import Alert from '@mui/material/Alert';
import Snackbar from '@mui/material/Snackbar';

// ----------------------------------------------------------------------

export interface SnackbarContextValue {
  showSnackbar: (message: string, severity?: AlertColor) => void;
}

export const SnackbarContext = createContext<SnackbarContextValue | undefined>(undefined);

// ----------------------------------------------------------------------

interface SnackbarState {
  open: boolean;
  message: string;
  severity: AlertColor;
}

interface SnackbarProviderProps {
  children: ReactNode;
}

export function SnackbarProvider({ children }: SnackbarProviderProps) {
  const [snackbar, setSnackbar] = useState<SnackbarState>({
    open: false,
    message: '',
    severity: 'info',
  });

  const showSnackbar = useCallback((message: string, severity: AlertColor = 'info') => {
    setSnackbar({
      open: true,
      message,
      severity,
    });
  }, []);

  const handleClose = useCallback(() => {
    setSnackbar((prev) => ({ ...prev, open: false }));
  }, []);

  return (
    <SnackbarContext.Provider value={{ showSnackbar }}>
      {children}
      <Snackbar
        open={snackbar.open}
        autoHideDuration={6000}
        onClose={handleClose}
        anchorOrigin={{ vertical: 'top', horizontal: 'right' }}
      >
        <Alert onClose={handleClose} severity={snackbar.severity} variant="filled" sx={{ width: '100%' }}>
          {snackbar.message}
        </Alert>
      </Snackbar>
    </SnackbarContext.Provider>
  );
}
