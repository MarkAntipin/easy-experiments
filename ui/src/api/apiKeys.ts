import { request } from './client';
import type {
  ApiKeySummary,
  CreateApiKeyResponse,
  MessageResponse,
} from './types';

export function listApiKeys(): Promise<ApiKeySummary[]> {
  return request<ApiKeySummary[]>('/admin/v1/api-keys');
}

export function createApiKey(name: string): Promise<CreateApiKeyResponse> {
  return request<CreateApiKeyResponse>('/admin/v1/api-keys', {
    method: 'POST',
    body: { name },
  });
}

export function revokeApiKey(id: string): Promise<MessageResponse> {
  return request<MessageResponse>(`/admin/v1/api-keys/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  });
}
