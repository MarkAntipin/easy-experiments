import { request } from './client';
import type { MessageResponse, UserListResponse, UserSummary } from './types';

export function listUsers(): Promise<UserListResponse> {
  return request<UserListResponse>('/admin/v1/users');
}

export function inviteUser(email: string): Promise<UserSummary> {
  return request<UserSummary>('/admin/v1/users', {
    method: 'POST',
    body: { email },
  });
}

export function removeUser(id: string): Promise<MessageResponse> {
  return request<MessageResponse>(`/admin/v1/users/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  });
}
