import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { AxonWebSocket } from '../axon-websocket';

// ----------------------------------------------------------------------

describe('AxonWebSocket', () => {
  let wsClient: AxonWebSocket;
  let mockWebSocket: any;

  beforeEach(() => {
    vi.useFakeTimers();
    wsClient = new AxonWebSocket();

    // Get the mock WebSocket instance
    mockWebSocket = (global.WebSocket as any).mock?.instances?.[0];
  });

  afterEach(() => {
    wsClient.disconnect();
    vi.clearAllMocks();
    vi.clearAllTimers();
    vi.useRealTimers();
  });

  // ----------------------------------------------------------------------
  // Connection
  // ----------------------------------------------------------------------

  describe('Connection', () => {
    it('should create WebSocket connection', () => {
      wsClient.connect();

      expect(WebSocket).toHaveBeenCalledWith('ws://127.0.0.1:9090/api/v1/ws');
    });

    it('should not create duplicate connection if already connected', () => {
      wsClient.connect();

      // Simulate connection opening
      vi.runAllTimers();

      const callCount = vi.mocked(WebSocket).mock.calls.length;

      wsClient.connect();

      expect(vi.mocked(WebSocket).mock.calls.length).toBe(callCount);
    });

    it('should set readyState to OPEN on successful connection', () => {
      wsClient.connect();

      // Fast-forward timers to trigger onopen
      vi.runAllTimers();

      expect(wsClient.isConnected()).toBe(true);
    });

    it('should handle connection errors', () => {
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      wsClient.connect();

      // Simulate error
      const ws = (wsClient as any).ws;
      if (ws && ws.onerror) {
        ws.onerror(new Event('error'));
      }

      expect(consoleErrorSpy).toHaveBeenCalled();

      consoleErrorSpy.mockRestore();
    });
  });

  // ----------------------------------------------------------------------
  // Disconnection
  // ----------------------------------------------------------------------

  describe('Disconnection', () => {
    it('should close WebSocket connection', () => {
      wsClient.connect();
      vi.runAllTimers();

      const ws = (wsClient as any).ws;
      const closeSpy = vi.spyOn(ws, 'close');

      wsClient.disconnect();

      expect(closeSpy).toHaveBeenCalled();
      expect((wsClient as any).isIntentionallyClosed).toBe(true);
    });

    it('should clear reconnect timeout on disconnect', () => {
      wsClient.connect();

      // Trigger a reconnection
      const ws = (wsClient as any).ws;
      if (ws && ws.onclose) {
        ws.onclose(new CloseEvent('close'));
      }

      const reconnectTimeout = (wsClient as any).reconnectTimeout;

      wsClient.disconnect();

      expect((wsClient as any).reconnectTimeout).toBeNull();
    });

    it('should reset reconnect attempts on disconnect', () => {
      wsClient.connect();

      wsClient.disconnect();

      expect((wsClient as any).reconnectAttempts).toBe(0);
    });

    it('should handle disconnect when not connected', () => {
      expect(() => wsClient.disconnect()).not.toThrow();
    });
  });

  // ----------------------------------------------------------------------
  // Reconnection
  // ----------------------------------------------------------------------

  describe('Reconnection', () => {
    it('should attempt to reconnect on connection loss', () => {
      wsClient.connect();
      vi.runAllTimers();

      const ws = (wsClient as any).ws;

      // Simulate connection close
      if (ws && ws.onclose) {
        ws.onclose(new CloseEvent('close'));
      }

      expect((wsClient as any).reconnectAttempts).toBe(1);
    });

    it('should not reconnect if intentionally closed', () => {
      wsClient.connect();
      vi.runAllTimers();

      wsClient.disconnect();

      expect((wsClient as any).reconnectAttempts).toBe(0);
    });

    it('should stop reconnecting after max attempts', () => {
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      wsClient.connect();

      // Simulate multiple failed connections
      for (let i = 0; i < 6; i++) {
        const ws = (wsClient as any).ws;
        if (ws && ws.onclose) {
          ws.onclose(new CloseEvent('close'));
        }
        vi.runAllTimers();
      }

      expect(consoleErrorSpy).toHaveBeenCalledWith(
        expect.stringContaining('Failed to reconnect after 5 attempts')
      );

      consoleErrorSpy.mockRestore();
    });

    it('should increase delay between reconnection attempts', () => {
      wsClient.connect();

      // First reconnection
      const ws1 = (wsClient as any).ws;
      if (ws1 && ws1.onclose) {
        ws1.onclose(new CloseEvent('close'));
      }

      expect((wsClient as any).reconnectAttempts).toBe(1);

      // Second reconnection
      vi.runAllTimers();
      const ws2 = (wsClient as any).ws;
      if (ws2 && ws2.onclose) {
        ws2.onclose(new CloseEvent('close'));
      }

      expect((wsClient as any).reconnectAttempts).toBe(2);
    });

    it('should reset reconnect attempts on successful connection', () => {
      wsClient.connect();

      // Simulate connection open
      const ws = (wsClient as any).ws;
      if (ws && ws.onopen) {
        ws.onopen(new Event('open'));
      }

      expect((wsClient as any).reconnectAttempts).toBe(0);
    });
  });

  // ----------------------------------------------------------------------
  // Message Handling
  // ----------------------------------------------------------------------

  describe('Message Handling', () => {
    it('should handle incoming messages', () => {
      const handler = vi.fn();
      wsClient.subscribe(handler);

      wsClient.connect();
      vi.runAllTimers();

      const mockEvent = {
        type: 'agent_status_changed',
        timestamp: '2024-01-01T00:00:00Z',
        data: { agent_id: 'agent-123', status: 'Working' },
      };

      const ws = (wsClient as any).ws;
      if (ws && ws.onmessage) {
        ws.onmessage({
          data: JSON.stringify(mockEvent),
        } as MessageEvent);
      }

      expect(handler).toHaveBeenCalledWith(mockEvent);
    });

    it('should handle malformed JSON messages', () => {
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      const handler = vi.fn();

      wsClient.subscribe(handler);
      wsClient.connect();
      vi.runAllTimers();

      const ws = (wsClient as any).ws;
      if (ws && ws.onmessage) {
        ws.onmessage({
          data: 'invalid json',
        } as MessageEvent);
      }

      expect(handler).not.toHaveBeenCalled();
      expect(consoleErrorSpy).toHaveBeenCalledWith(
        'Failed to parse WebSocket message:',
        expect.any(Error)
      );

      consoleErrorSpy.mockRestore();
    });

    it('should notify all subscribed handlers', () => {
      const handler1 = vi.fn();
      const handler2 = vi.fn();

      wsClient.subscribe(handler1);
      wsClient.subscribe(handler2);

      wsClient.connect();
      vi.runAllTimers();

      const mockEvent = {
        type: 'workflow_started',
        timestamp: '2024-01-01T00:00:00Z',
        data: { workflow_id: 'wf-123' },
      };

      const ws = (wsClient as any).ws;
      if (ws && ws.onmessage) {
        ws.onmessage({
          data: JSON.stringify(mockEvent),
        } as MessageEvent);
      }

      expect(handler1).toHaveBeenCalledWith(mockEvent);
      expect(handler2).toHaveBeenCalledWith(mockEvent);
    });

    it('should handle errors in event handlers gracefully', () => {
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      const errorHandler = vi.fn(() => {
        throw new Error('Handler error');
      });
      const normalHandler = vi.fn();

      wsClient.subscribe(errorHandler);
      wsClient.subscribe(normalHandler);

      wsClient.connect();
      vi.runAllTimers();

      const mockEvent = {
        type: 'agent_started',
        timestamp: '2024-01-01T00:00:00Z',
        data: { agent_id: 'agent-123' },
      };

      const ws = (wsClient as any).ws;
      if (ws && ws.onmessage) {
        ws.onmessage({
          data: JSON.stringify(mockEvent),
        } as MessageEvent);
      }

      expect(errorHandler).toHaveBeenCalled();
      expect(normalHandler).toHaveBeenCalled();
      expect(consoleErrorSpy).toHaveBeenCalled();

      consoleErrorSpy.mockRestore();
    });
  });

  // ----------------------------------------------------------------------
  // Subscription
  // ----------------------------------------------------------------------

  describe('Subscription', () => {
    it('should subscribe to events', () => {
      const handler = vi.fn();
      const unsubscribe = wsClient.subscribe(handler);

      expect(typeof unsubscribe).toBe('function');
    });

    it('should unsubscribe from events', () => {
      const handler = vi.fn();
      const unsubscribe = wsClient.subscribe(handler);

      wsClient.connect();
      vi.runAllTimers();

      // Unsubscribe
      unsubscribe();

      const mockEvent = {
        type: 'agent_paused',
        timestamp: '2024-01-01T00:00:00Z',
        data: { agent_id: 'agent-123' },
      };

      const ws = (wsClient as any).ws;
      if (ws && ws.onmessage) {
        ws.onmessage({
          data: JSON.stringify(mockEvent),
        } as MessageEvent);
      }

      expect(handler).not.toHaveBeenCalled();
    });

    it('should support multiple subscriptions', () => {
      const handler1 = vi.fn();
      const handler2 = vi.fn();

      wsClient.subscribe(handler1);
      wsClient.subscribe(handler2);

      expect((wsClient as any).handlers.size).toBe(2);
    });

    it('should allow selective unsubscription', () => {
      const handler1 = vi.fn();
      const handler2 = vi.fn();

      const unsubscribe1 = wsClient.subscribe(handler1);
      wsClient.subscribe(handler2);

      unsubscribe1();

      wsClient.connect();
      vi.runAllTimers();

      const mockEvent = {
        type: 'task_completed',
        timestamp: '2024-01-01T00:00:00Z',
        data: { task_id: 'task-123' },
      };

      const ws = (wsClient as any).ws;
      if (ws && ws.onmessage) {
        ws.onmessage({
          data: JSON.stringify(mockEvent),
        } as MessageEvent);
      }

      expect(handler1).not.toHaveBeenCalled();
      expect(handler2).toHaveBeenCalledWith(mockEvent);
    });
  });

  // ----------------------------------------------------------------------
  // Send Messages
  // ----------------------------------------------------------------------

  describe('Send Messages', () => {
    it('should send messages when connected', () => {
      wsClient.connect();
      vi.runAllTimers();

      const ws = (wsClient as any).ws;
      const sendSpy = vi.spyOn(ws, 'send');

      const message = { action: 'ping' };
      wsClient.send(message);

      expect(sendSpy).toHaveBeenCalledWith(JSON.stringify(message));
    });

    it('should not send messages when not connected', () => {
      const consoleWarnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

      const message = { action: 'ping' };
      wsClient.send(message);

      expect(consoleWarnSpy).toHaveBeenCalledWith(
        'WebSocket not connected, cannot send message'
      );

      consoleWarnSpy.mockRestore();
    });

    it('should handle send errors gracefully', () => {
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      wsClient.connect();
      vi.runAllTimers();

      const ws = (wsClient as any).ws;
      vi.spyOn(ws, 'send').mockImplementation(() => {
        throw new Error('Send failed');
      });

      wsClient.send({ action: 'test' });

      expect(consoleErrorSpy).toHaveBeenCalledWith(
        'Failed to send WebSocket message:',
        expect.any(Error)
      );

      consoleErrorSpy.mockRestore();
    });
  });

  // ----------------------------------------------------------------------
  // Connection Status
  // ----------------------------------------------------------------------

  describe('Connection Status', () => {
    it('should return false when not connected', () => {
      expect(wsClient.isConnected()).toBe(false);
    });

    it('should return true when connected', () => {
      wsClient.connect();
      vi.runAllTimers();

      expect(wsClient.isConnected()).toBe(true);
    });

    it('should return false after disconnection', () => {
      wsClient.connect();
      vi.runAllTimers();

      wsClient.disconnect();

      expect(wsClient.isConnected()).toBe(false);
    });
  });
});
