import { request } from './client';
import type {
  ApiKeyListResponse,
  CreateApiKeyResponse,
  MessageResponse,
} from './types';

export function listApiKeys(): Promise<ApiKeyListResponse> {
  return request<ApiKeyListResponse>('/admin/v1/api-keys');
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
