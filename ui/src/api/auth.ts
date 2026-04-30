import { request } from './client';
import type { LoginResponse } from './types';

export function googleLogin(idToken: string): Promise<LoginResponse> {
  return request<LoginResponse>('/admin/v1/auth/google', {
    method: 'POST',
    body: { token: idToken },
    unauthenticated: true,
  });
}
