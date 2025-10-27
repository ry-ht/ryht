import axios from 'axios';
import { it, vi, expect, describe, afterEach, beforeEach } from 'vitest';

import { JWT_STORAGE_KEY } from '../constant';
import { jwtDecode, setSession, isValidToken } from '../utils';

// ----------------------------------------------------------------------
// Mock dependencies
// ----------------------------------------------------------------------

vi.mock('src/lib/axios', () => ({
  default: {
    defaults: {
      headers: {
        common: {} as Record<string, string>,
      },
    },
  },
}));

// Mock window.alert
global.alert = vi.fn();

// ----------------------------------------------------------------------

describe('Auth Utils', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    sessionStorage.clear();
    vi.useFakeTimers();
    // Reset axios headers
    axios.defaults.headers.common = {};
  });

  afterEach(() => {
    vi.clearAllTimers();
    vi.useRealTimers();
  });

  // ----------------------------------------------------------------------
  // jwtDecode
  // ----------------------------------------------------------------------

  describe('jwtDecode', () => {
    it('should decode a valid JWT token', () => {
      // Create a mock JWT token (header.payload.signature)
      const payload = { sub: '1234567890', name: 'John Doe', exp: 9999999999 };
      const base64Payload = btoa(JSON.stringify(payload));
      const mockToken = `header.${base64Payload}.signature`;

      const decoded = jwtDecode(mockToken);

      expect(decoded).toEqual(payload);
    });

    it('should return null for empty token', () => {
      const result = jwtDecode('');

      expect(result).toBeNull();
    });

    it('should throw error for invalid token format', () => {
      expect(() => jwtDecode('invalid-token')).toThrow('Invalid token!');
    });

    it('should handle URL-safe base64 encoding', () => {
      const payload = { sub: '1234567890', data: 'test+data/value=' };
      const base64Payload = btoa(JSON.stringify(payload)).replace(/\+/g, '-').replace(/\//g, '_');
      const mockToken = `header.${base64Payload}.signature`;

      const decoded = jwtDecode(mockToken);

      expect(decoded).toBeDefined();
    });

    it('should throw error for malformed JSON in payload', () => {
      const mockToken = 'header.invalid-base64.signature';

      expect(() => jwtDecode(mockToken)).toThrow();
    });
  });

  // ----------------------------------------------------------------------
  // isValidToken
  // ----------------------------------------------------------------------

  describe('isValidToken', () => {
    it('should return true for valid non-expired token', () => {
      const futureExp = Math.floor(Date.now() / 1000) + 3600; // 1 hour from now
      const payload = { sub: '123', exp: futureExp };
      const base64Payload = btoa(JSON.stringify(payload));
      const mockToken = `header.${base64Payload}.signature`;

      const result = isValidToken(mockToken);

      expect(result).toBe(true);
    });

    it('should return false for expired token', () => {
      const pastExp = Math.floor(Date.now() / 1000) - 3600; // 1 hour ago
      const payload = { sub: '123', exp: pastExp };
      const base64Payload = btoa(JSON.stringify(payload));
      const mockToken = `header.${base64Payload}.signature`;

      const result = isValidToken(mockToken);

      expect(result).toBe(false);
    });

    it('should return false for empty token', () => {
      const result = isValidToken('');

      expect(result).toBe(false);
    });

    it('should return false for token without exp claim', () => {
      const payload = { sub: '123' }; // No exp
      const base64Payload = btoa(JSON.stringify(payload));
      const mockToken = `header.${base64Payload}.signature`;

      const result = isValidToken(mockToken);

      expect(result).toBe(false);
    });

    it('should return false for malformed token', () => {
      const result = isValidToken('invalid-token-format');

      expect(result).toBe(false);
    });

    it('should return true for token expiring in the future', () => {
      const futureExp = Math.floor(Date.now() / 1000) + 86400; // 24 hours from now
      const payload = { sub: '123', exp: futureExp };
      const base64Payload = btoa(JSON.stringify(payload));
      const mockToken = `header.${base64Payload}.signature`;

      const result = isValidToken(mockToken);

      expect(result).toBe(true);
    });
  });

  // ----------------------------------------------------------------------
  // setSession
  // ----------------------------------------------------------------------

  describe('setSession', () => {
    it('should set session with valid token', async () => {
      const futureExp = Math.floor(Date.now() / 1000) + 3600;
      const payload = { sub: '123', exp: futureExp };
      const base64Payload = btoa(JSON.stringify(payload));
      const mockToken = `header.${base64Payload}.signature`;

      await setSession(mockToken);

      expect(sessionStorage.getItem(JWT_STORAGE_KEY)).toBe(mockToken);
      expect(axios.defaults.headers.common.Authorization).toBe(`Bearer ${mockToken}`);
    });

    it('should clear session when token is null', async () => {
      // First set a token
      const futureExp = Math.floor(Date.now() / 1000) + 3600;
      const payload = { sub: '123', exp: futureExp };
      const base64Payload = btoa(JSON.stringify(payload));
      const mockToken = `header.${base64Payload}.signature`;

      await setSession(mockToken);

      // Then clear it
      await setSession(null);

      expect(sessionStorage.getItem(JWT_STORAGE_KEY)).toBeNull();
      expect(axios.defaults.headers.common.Authorization).toBeUndefined();
    });

    it('should throw error for invalid token without exp', async () => {
      const payload = { sub: '123' }; // No exp
      const base64Payload = btoa(JSON.stringify(payload));
      const mockToken = `header.${base64Payload}.signature`;

      await expect(setSession(mockToken)).rejects.toThrow('Invalid access token!');
    });

    it('should store token in sessionStorage', async () => {
      const futureExp = Math.floor(Date.now() / 1000) + 7200;
      const payload = { sub: '123', exp: futureExp };
      const base64Payload = btoa(JSON.stringify(payload));
      const mockToken = `header.${base64Payload}.signature`;

      await setSession(mockToken);

      const storedToken = sessionStorage.getItem(JWT_STORAGE_KEY);
      expect(storedToken).toBe(mockToken);
    });

    it('should set Authorization header correctly', async () => {
      const futureExp = Math.floor(Date.now() / 1000) + 3600;
      const payload = { sub: '123', exp: futureExp };
      const base64Payload = btoa(JSON.stringify(payload));
      const mockToken = `header.${base64Payload}.signature`;

      await setSession(mockToken);

      expect(axios.defaults.headers.common.Authorization).toBe(`Bearer ${mockToken}`);
    });

    it('should remove Authorization header when clearing session', async () => {
      // Set token first
      const futureExp = Math.floor(Date.now() / 1000) + 3600;
      const payload = { sub: '123', exp: futureExp };
      const base64Payload = btoa(JSON.stringify(payload));
      const mockToken = `header.${base64Payload}.signature`;

      await setSession(mockToken);
      expect(axios.defaults.headers.common.Authorization).toBeDefined();

      // Clear session
      await setSession(null);
      expect(axios.defaults.headers.common.Authorization).toBeUndefined();
    });

    it('should handle malformed token gracefully', async () => {
      await expect(setSession('malformed-token')).rejects.toThrow();
    });
  });

  // ----------------------------------------------------------------------
  // Integration tests
  // ----------------------------------------------------------------------

  describe('Integration: Token lifecycle', () => {
    it('should handle complete token lifecycle', async () => {
      // Create a valid token
      const futureExp = Math.floor(Date.now() / 1000) + 3600;
      const payload = { sub: '123', exp: futureExp, name: 'Test User' };
      const base64Payload = btoa(JSON.stringify(payload));
      const mockToken = `header.${base64Payload}.signature`;

      // Verify token is valid
      expect(isValidToken(mockToken)).toBe(true);

      // Set session
      await setSession(mockToken);

      // Verify storage and headers
      expect(sessionStorage.getItem(JWT_STORAGE_KEY)).toBe(mockToken);
      expect(axios.defaults.headers.common.Authorization).toBe(`Bearer ${mockToken}`);

      // Decode and verify
      const decoded = jwtDecode(mockToken);
      expect(decoded).toMatchObject(payload);

      // Clear session
      await setSession(null);

      // Verify cleanup
      expect(sessionStorage.getItem(JWT_STORAGE_KEY)).toBeNull();
      expect(axios.defaults.headers.common.Authorization).toBeUndefined();
    });

    it('should reject expired token in full lifecycle', async () => {
      // Create an expired token
      const pastExp = Math.floor(Date.now() / 1000) - 3600;
      const payload = { sub: '123', exp: pastExp };
      const base64Payload = btoa(JSON.stringify(payload));
      const mockToken = `header.${base64Payload}.signature`;

      // Verify token is invalid
      expect(isValidToken(mockToken)).toBe(false);

      // Token can still be decoded
      const decoded = jwtDecode(mockToken);
      expect(decoded).toMatchObject(payload);
    });
  });
});
