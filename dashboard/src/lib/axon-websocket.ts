import type { WebSocketEvent } from '../types/axon';

// ----------------------------------------------------------------------

const AXON_WS_URL = import.meta.env.VITE_AXON_WS_URL || 'ws://127.0.0.1:9090/api/v1/ws';
const RECONNECT_DELAY = 3000;
const MAX_RECONNECT_ATTEMPTS = 5;

// ----------------------------------------------------------------------

export type WebSocketEventHandler = (event: WebSocketEvent) => void;

export class AxonWebSocket {
  private ws: WebSocket | null = null;
  private handlers: Set<WebSocketEventHandler> = new Set();
  private reconnectAttempts = 0;
  private reconnectTimeout: NodeJS.Timeout | null = null;
  private isIntentionallyClosed = false;

  /**
   * Connect to Axon WebSocket server
   */
  connect(): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      console.log('WebSocket already connected');
      return;
    }

    this.isIntentionallyClosed = false;

    try {
      this.ws = new WebSocket(AXON_WS_URL);

      this.ws.onopen = () => {
        console.log('WebSocket connected to Axon');
        this.reconnectAttempts = 0;
      };

      this.ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data) as WebSocketEvent;
          this.notifyHandlers(data);
        } catch (error) {
          console.error('Failed to parse WebSocket message:', error);
        }
      };

      this.ws.onerror = (error) => {
        console.error('WebSocket error:', error);
      };

      this.ws.onclose = () => {
        console.log('WebSocket disconnected');
        this.ws = null;

        if (!this.isIntentionallyClosed) {
          this.scheduleReconnect();
        }
      };
    } catch (error) {
      console.error('Failed to create WebSocket connection:', error);
      this.scheduleReconnect();
    }
  }

  /**
   * Disconnect from WebSocket server
   */
  disconnect(): void {
    this.isIntentionallyClosed = true;

    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
      this.reconnectTimeout = null;
    }

    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }

    this.reconnectAttempts = 0;
  }

  /**
   * Subscribe to WebSocket events
   */
  subscribe(handler: WebSocketEventHandler): () => void {
    this.handlers.add(handler);

    // Return unsubscribe function
    return () => {
      this.handlers.delete(handler);
    };
  }

  /**
   * Check if WebSocket is connected
   */
  isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  /**
   * Send a message to the server
   */
  send(data: any): void {
    if (!this.isConnected()) {
      console.warn('WebSocket not connected, cannot send message');
      return;
    }

    try {
      this.ws?.send(JSON.stringify(data));
    } catch (error) {
      console.error('Failed to send WebSocket message:', error);
    }
  }

  /**
   * Schedule reconnection attempt
   */
  private scheduleReconnect(): void {
    if (this.reconnectAttempts >= MAX_RECONNECT_ATTEMPTS) {
      console.error(`Failed to reconnect after ${MAX_RECONNECT_ATTEMPTS} attempts`);
      return;
    }

    this.reconnectAttempts += 1;
    const delay = RECONNECT_DELAY * this.reconnectAttempts;

    console.log(`Scheduling reconnect attempt ${this.reconnectAttempts} in ${delay}ms`);

    this.reconnectTimeout = setTimeout(() => {
      console.log(`Reconnecting... (attempt ${this.reconnectAttempts})`);
      this.connect();
    }, delay);
  }

  /**
   * Notify all registered handlers of an event
   */
  private notifyHandlers(event: WebSocketEvent): void {
    this.handlers.forEach((handler) => {
      try {
        handler(event);
      } catch (error) {
        console.error('Error in WebSocket event handler:', error);
      }
    });
  }
}

// ----------------------------------------------------------------------
// Singleton instance
// ----------------------------------------------------------------------

export const axonWebSocket = new AxonWebSocket();

// ----------------------------------------------------------------------
// React Hook for WebSocket
// ----------------------------------------------------------------------

export const useAxonWebSocket = (handler: WebSocketEventHandler) => {
  const [isConnected, setIsConnected] = React.useState(false);

  React.useEffect(() => {
    // Connect to WebSocket
    axonWebSocket.connect();

    // Subscribe to events
    const unsubscribe = axonWebSocket.subscribe(handler);

    // Check connection status
    const checkConnection = setInterval(() => {
      setIsConnected(axonWebSocket.isConnected());
    }, 1000);

    // Cleanup
    return () => {
      unsubscribe();
      clearInterval(checkConnection);
    };
  }, [handler]);

  return { isConnected };
};

// Add React import for the hook
import React from 'react';
