/**
 * Shared error types and utilities for frontend API error handling.
 * Matches the standardized backend error format: { error: { code, message } }
 */

export interface ApiErrorResponse {
  error: {
    code: string;
    message: string;
  };
}

export type ErrorCode =
  | 'INTERNAL_ERROR'
  | 'BAD_REQUEST'
  | 'UNAUTHORIZED'
  | 'CONFLICT'
  | 'NOT_FOUND'
  | 'RATE_LIMITED'
  | 'UPSTREAM_ERROR'
  | 'SERVICE_UNAVAILABLE'
  | 'POLICY_VIOLATION'
  | 'AGENT_LOOP_DETECTED'
  | 'BURN_RATE_EXCEEDED'
  | 'COMPLIANCE_ERROR';

export interface StandardizedError {
  code: ErrorCode;
  message: string;
  status?: number;
}

/**
 * Parses a fetch response into a standardized error object.
 * Handles the backend's standardized error format.
 */
export async function parseApiError(response: Response): Promise<StandardizedError> {
  const status = response.status;
  
  try {
    const data = await response.json() as ApiErrorResponse;
    
    // Check if it matches our standardized format
    if (data.error && typeof data.error.code === 'string' && typeof data.error.message === 'string') {
      return {
        code: data.error.code as ErrorCode,
        message: data.error.message,
        status,
      };
    }
    
    // Fallback for legacy error formats
    if (data.error && typeof data.error === 'string') {
      return {
        code: 'INTERNAL_ERROR',
        message: data.error,
        status,
      };
    }
    
    // Check for message field directly on data
    if ('message' in data && typeof data.message === 'string') {
      return {
        code: 'INTERNAL_ERROR',
        message: data.message,
        status,
      };
    }
  } catch (e) {
    // JSON parsing failed, use status text
  }
  
  // Fallback to HTTP status text
  return {
    code: mapHttpStatusToErrorCode(status),
    message: response.statusText || 'An unexpected error occurred',
    status,
  };
}

/**
 * Maps HTTP status codes to error codes.
 */
function mapHttpStatusToErrorCode(status: number): ErrorCode {
  switch (status) {
    case 400:
      return 'BAD_REQUEST';
    case 401:
      return 'UNAUTHORIZED';
    case 403:
      return 'UNAUTHORIZED';
    case 404:
      return 'NOT_FOUND';
    case 409:
      return 'CONFLICT';
    case 422:
      return 'POLICY_VIOLATION';
    case 429:
      return 'RATE_LIMITED';
    case 502:
    case 503:
    case 504:
      return 'SERVICE_UNAVAILABLE';
    default:
      return 'INTERNAL_ERROR';
  }
}

/**
 * Determines if an error is a authentication error that should trigger logout.
 */
export function isAuthError(error: StandardizedError): boolean {
  return error.code === 'UNAUTHORIZED' || error.status === 401;
}

/**
 * Determines if an error is retryable.
 */
export function isRetryableError(error: StandardizedError): boolean {
  return ['INTERNAL_ERROR', 'UPSTREAM_ERROR', 'SERVICE_UNAVAILABLE'].includes(error.code) ||
    (error.status !== undefined && error.status >= 500);
}

/**
 * Formats an error for display in the UI.
 */
export function formatErrorForDisplay(error: StandardizedError): string {
  // Return the server-provided message (already sanitized by backend)
  return error.message;
}
