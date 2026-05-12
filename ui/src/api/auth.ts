import { request } from './client';
import type { LoginResponse } from './types';

export function googleLogin(idToken: string): Promise<LoginResponse> {
  return request<LoginResponse>('/admin/v1/auth/google', {
    method: 'POST',
    body: { token: idToken },
    unauthenticated: true,
  });
}

export function passwordLogin(
  email: string,
  password: string,
): Promise<LoginResponse> {
  return request<LoginResponse>('/admin/v1/auth/login', {
    method: 'POST',
    body: { email, password },
    unauthenticated: true,
  });
}

export function acceptInvite(
  token: string,
  password: string,
): Promise<LoginResponse> {
  return request<LoginResponse>('/admin/v1/auth/accept-invite', {
    method: 'POST',
    body: { token, password },
    unauthenticated: true,
  });
}
