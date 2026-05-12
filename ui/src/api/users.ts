import { request } from './client';
import type { InviteUserResponse, MessageResponse, UserListResponse } from './types';

export function listUsers(): Promise<UserListResponse> {
  return request<UserListResponse>('/admin/v1/users');
}

export function inviteUser(email: string): Promise<InviteUserResponse> {
  return request<InviteUserResponse>('/admin/v1/users', {
    method: 'POST',
    body: { email },
  });
}

export function removeUser(id: string): Promise<MessageResponse> {
  return request<MessageResponse>(`/admin/v1/users/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  });
}
