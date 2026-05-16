import type { ApiErrorBody } from './types';

const TOKEN_STORAGE_KEY = 'ee.auth.token';

export class ApiError extends Error {
  readonly status: number;
  readonly body: ApiErrorBody | null;

  constructor(status: number, message: string, body: ApiErrorBody | null) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
    this.body = body;
  }

  get isUnauthorized(): boolean {
    return this.status === 401;
  }

  get isConflict(): boolean {
    return this.status === 409 || this.status === 412;
  }
}

function baseUrl(): string {
  // UI and API are always served from the same origin: the Vite dev server
  // proxies /admin, /api, /health, /config.js to the backend in dev, and the
  // bundled Docker image serves UI + API on the same port in prod.
  return '';
}

export function getStoredToken(): string | null {
  return localStorage.getItem(TOKEN_STORAGE_KEY);
}

export function setStoredToken(token: string | null): void {
  if (token === null) {
    localStorage.removeItem(TOKEN_STORAGE_KEY);
  } else {
    localStorage.setItem(TOKEN_STORAGE_KEY, token);
  }
}

type UnauthorizedHandler = () => void;
let onUnauthorized: UnauthorizedHandler | null = null;

export function setUnauthorizedHandler(handler: UnauthorizedHandler | null): void {
  onUnauthorized = handler;
}

interface RequestOptions {
  method?: 'GET' | 'POST' | 'PATCH' | 'DELETE';
  body?: unknown;
  headers?: Record<string, string>;
  query?: Record<string, string | number | boolean | undefined>;
  signal?: AbortSignal;
  /** If true, do NOT attach the stored JWT to the request. */
  unauthenticated?: boolean;
}

function buildQuery(query?: RequestOptions['query']): string {
  if (!query) return '';
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(query)) {
    if (value === undefined || value === null) continue;
    params.set(key, String(value));
  }
  const s = params.toString();
  return s ? `?${s}` : '';
}

export async function request<T>(
  path: string,
  options: RequestOptions = {},
): Promise<T> {
  const { method = 'GET', body, headers = {}, query, signal, unauthenticated } = options;

  const finalHeaders: Record<string, string> = {
    Accept: 'application/json',
    ...headers,
  };

  if (body !== undefined && !('Content-Type' in finalHeaders)) {
    finalHeaders['Content-Type'] = 'application/json';
  }

  if (!unauthenticated) {
    const token = getStoredToken();
    if (token) {
      finalHeaders.Authorization = `Bearer ${token}`;
    }
  }

  const url = `${baseUrl()}${path}${buildQuery(query)}`;

  let response: Response;
  try {
    response = await fetch(url, {
      method,
      headers: finalHeaders,
      body: body === undefined ? undefined : JSON.stringify(body),
      signal,
    });
  } catch (err) {
    if (err instanceof DOMException && err.name === 'AbortError') {
      throw err;
    }
    throw new ApiError(0, 'Network error. Is the API reachable?', null);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  const text = await response.text();
  let parsed: unknown = null;
  if (text.length > 0) {
    try {
      parsed = JSON.parse(text);
    } catch {
      parsed = { message: text };
    }
  }

  if (!response.ok) {
    const errBody = (parsed ?? null) as ApiErrorBody | null;
    const message =
      (errBody && typeof errBody.message === 'string' && errBody.message) ||
      `Request failed with status ${response.status}`;
    if (response.status === 401 && onUnauthorized) {
      onUnauthorized();
    }
    throw new ApiError(response.status, message, errBody);
  }

  return parsed as T;
}
