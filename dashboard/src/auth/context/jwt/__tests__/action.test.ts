import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import axios from 'axios';
import { signInWithPassword, signUp, signOut } from '../action';
import { setSession } from '../utils';

// ----------------------------------------------------------------------
// Mock dependencies
// ----------------------------------------------------------------------

vi.mock('src/lib/axios', () => ({
  default: {
    post: vi.fn(),
  },
  endpoints: {
    auth: {
      signIn: '/api/auth/signin',
      signUp: '/api/auth/signup',
    },
  },
}));

vi.mock('../utils', () => ({
  setSession: vi.fn(),
}));

// ----------------------------------------------------------------------

describe('Auth Actions', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    sessionStorage.clear();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  // ----------------------------------------------------------------------
  // signInWithPassword
  // ----------------------------------------------------------------------

  describe('signInWithPassword', () => {
    it('should sign in successfully with valid credentials', async () => {
      const mockAccessToken = 'mock-access-token-12345';
      const mockResponse = { data: { accessToken: mockAccessToken } };

      vi.mocked(axios.post).mockResolvedValueOnce(mockResponse);

      await signInWithPassword({
        email: 'test@example.com',
        password: 'password123',
      });

      expect(vi.mocked(axios.post)).toHaveBeenCalledWith('/api/auth/signin', {
        email: 'test@example.com',
        password: 'password123',
      });

      expect(vi.mocked(setSession)).toHaveBeenCalledWith(mockAccessToken);
    });

    it('should throw error when access token is missing', async () => {
      const mockResponse = { data: {} };

      vi.mocked(axios.post).mockResolvedValueOnce(mockResponse);

      await expect(
        signInWithPassword({
          email: 'test@example.com',
          password: 'password123',
        })
      ).rejects.toThrow('Access token not found in response');
    });

    it('should throw error when API request fails', async () => {
      const mockError = new Error('Network error');

      vi.mocked(axios.post).mockRejectedValueOnce(mockError);

      await expect(
        signInWithPassword({
          email: 'test@example.com',
          password: 'password123',
        })
      ).rejects.toThrow('Network error');

      expect(vi.mocked(setSession)).not.toHaveBeenCalled();
    });

    it('should handle server error responses', async () => {
      const mockError = {
        response: {
          data: { message: 'Invalid credentials' },
          status: 401,
        },
      };

      vi.mocked(axios.post).mockRejectedValueOnce(mockError);

      await expect(
        signInWithPassword({
          email: 'test@example.com',
          password: 'wrongpassword',
        })
      ).rejects.toMatchObject(mockError);
    });

    it('should call setSession with correct token', async () => {
      const mockAccessToken = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test';
      const mockResponse = { data: { accessToken: mockAccessToken } };

      vi.mocked(axios.post).mockResolvedValueOnce(mockResponse);

      await signInWithPassword({
        email: 'admin@test.com',
        password: 'admin123',
      });

      expect(vi.mocked(setSession)).toHaveBeenCalledTimes(1);
      expect(vi.mocked(setSession)).toHaveBeenCalledWith(mockAccessToken);
    });
  });

  // ----------------------------------------------------------------------
  // signUp
  // ----------------------------------------------------------------------

  describe('signUp', () => {
    it('should sign up successfully with valid data', async () => {
      const mockAccessToken = 'mock-access-token-signup';
      const mockResponse = { data: { accessToken: mockAccessToken } };

      vi.mocked(axios.post).mockResolvedValueOnce(mockResponse);

      await signUp({
        email: 'newuser@example.com',
        password: 'password123',
        firstName: 'John',
        lastName: 'Doe',
      });

      expect(axios.post).toHaveBeenCalledWith('/api/auth/signup', {
        email: 'newuser@example.com',
        password: 'password123',
        firstName: 'John',
        lastName: 'Doe',
      });

      expect(vi.mocked(setSession)).toHaveBeenCalledWith(mockAccessToken);
    });

    it('should throw error when access token is missing in signup response', async () => {
      const mockResponse = { data: {} };

      vi.mocked(axios.post).mockResolvedValueOnce(mockResponse);

      await expect(
        signUp({
          email: 'newuser@example.com',
          password: 'password123',
          firstName: 'John',
          lastName: 'Doe',
        })
      ).rejects.toThrow('Access token not found in response');
    });

    it('should handle validation errors', async () => {
      const mockError = {
        response: {
          data: { message: 'Email already exists' },
          status: 400,
        },
      };

      vi.mocked(axios.post).mockRejectedValueOnce(mockError);

      await expect(
        signUp({
          email: 'existing@example.com',
          password: 'password123',
          firstName: 'John',
          lastName: 'Doe',
        })
      ).rejects.toMatchObject(mockError);

      expect(vi.mocked(setSession)).not.toHaveBeenCalled();
    });

    it('should handle network errors during signup', async () => {
      const mockError = new Error('Network connection failed');

      vi.mocked(axios.post).mockRejectedValueOnce(mockError);

      await expect(
        signUp({
          email: 'newuser@example.com',
          password: 'password123',
          firstName: 'Jane',
          lastName: 'Smith',
        })
      ).rejects.toThrow('Network connection failed');
    });
  });

  // ----------------------------------------------------------------------
  // signOut
  // ----------------------------------------------------------------------

  describe('signOut', () => {
    it('should sign out successfully', async () => {
      await signOut();

      expect(vi.mocked(setSession)).toHaveBeenCalledWith(null);
    });

    it('should handle errors during sign out', async () => {
      const mockError = new Error('Failed to clear session');

      vi.mocked(setSession).mockRejectedValueOnce(mockError);

      await expect(signOut()).rejects.toThrow('Failed to clear session');
    });

    it('should clear session even if already signed out', async () => {
      // Sign out when not signed in
      await signOut();

      expect(vi.mocked(setSession)).toHaveBeenCalledWith(null);
      expect(vi.mocked(setSession)).toHaveBeenCalledTimes(1);
    });
  });

  // ----------------------------------------------------------------------
  // Integration tests
  // ----------------------------------------------------------------------

  describe('Integration: Sign in and sign out flow', () => {
    it('should complete full authentication cycle', async () => {
      // Sign in
      const mockAccessToken = 'mock-token-integration';
      const mockResponse = { data: { accessToken: mockAccessToken } };

      vi.mocked(axios.post).mockResolvedValueOnce(mockResponse);

      await signInWithPassword({
        email: 'test@example.com',
        password: 'password123',
      });

      expect(vi.mocked(setSession)).toHaveBeenCalledWith(mockAccessToken);

      // Sign out
      vi.mocked(setSession).mockClear();

      await signOut();

      expect(vi.mocked(setSession)).toHaveBeenCalledWith(null);
    });
  });
});
