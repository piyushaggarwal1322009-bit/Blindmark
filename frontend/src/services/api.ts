import type { ApiFailure, SystemHealthResponse } from '../types';

export interface BlindMarkApi {
  checkHealth(baseUrl: string, signal?: AbortSignal): Promise<SystemHealthResponse>;
}

export class ApiRequestError extends Error {
  constructor(public readonly failure: ApiFailure) {
    super(failure.message);
    this.name = 'ApiRequestError';
  }
}

export const blindMarkApi: BlindMarkApi = {
  async checkHealth(baseUrl, externalSignal) {
    if (!baseUrl.trim()) {
      throw new ApiRequestError({ kind: 'not-configured', message: 'Set an API base URL to test the confirmed FastAPI health endpoint.' });
    }

    let endpoint: URL;
    try {
      endpoint = new URL('/healthz', baseUrl);
    } catch {
      throw new ApiRequestError({ kind: 'unexpected', message: 'Enter a valid absolute API URL, including its scheme.' });
    }

    const controller = new AbortController();
    const timeout = window.setTimeout(() => controller.abort(new Error('Request timed out')), 4000);
    const abortFromCaller = () => controller.abort(externalSignal?.reason);
    externalSignal?.addEventListener('abort', abortFromCaller, { once: true });

    try {
      const response = await fetch(endpoint, { method: 'GET', headers: { Accept: 'application/json' }, signal: controller.signal });
      if (response.status === 401 || response.status === 403) {
        throw new ApiRequestError({ kind: 'unauthorized', message: 'The health endpoint rejected the request.' });
      }
      if (!response.ok) {
        throw new ApiRequestError({ kind: 'unexpected', message: `Health endpoint returned HTTP ${response.status}.` });
      }
      const payload: unknown = await response.json();
      if (!isSystemHealthResponse(payload)) {
        throw new ApiRequestError({ kind: 'unexpected', message: 'The endpoint response does not match the confirmed BlindMark health contract.' });
      }
      return payload;
    } catch (error) {
      if (error instanceof ApiRequestError) throw error;
      if (controller.signal.aborted) {
        throw new ApiRequestError({ kind: 'timeout', message: 'The health check timed out or was cancelled.' });
      }
      throw new ApiRequestError({ kind: 'network', message: 'Could not reach the health endpoint. Check the URL, service availability, and CORS policy.' });
    } finally {
      window.clearTimeout(timeout);
      externalSignal?.removeEventListener('abort', abortFromCaller);
    }
  },
};

function isSystemHealthResponse(value: unknown): value is SystemHealthResponse {
  return typeof value === 'object' && value !== null && 'status' in value && value.status === 'ok' && 'service' in value && value.service === 'blindmark-backend';
}
