import { request } from './client';
import type {
  CreateExperimentRequest,
  CreateExperimentResponse,
  ExperimentDetail,
  ExperimentStatus,
  ExperimentSummary,
  MessageResponse,
  UpdateExperimentRequest,
} from './types';

export function listExperiments(
  status?: Exclude<ExperimentStatus, 'deleted'>,
): Promise<ExperimentSummary[]> {
  return request<ExperimentSummary[]>('/admin/v1/experiments', {
    query: status ? { status } : undefined,
  });
}

export function getExperiment(id: string): Promise<ExperimentDetail> {
  return request<ExperimentDetail>(`/admin/v1/experiments/${encodeURIComponent(id)}`);
}

export function createExperiment(
  body: CreateExperimentRequest,
): Promise<CreateExperimentResponse> {
  return request<CreateExperimentResponse>('/admin/v1/experiments', {
    method: 'POST',
    body,
  });
}

export function updateExperiment(
  id: string,
  body: UpdateExperimentRequest,
  updatedAt?: number,
): Promise<MessageResponse> {
  const headers: Record<string, string> = {};
  if (updatedAt !== undefined) {
    headers['If-Match'] = `"${updatedAt}"`;
  }
  return request<MessageResponse>(`/admin/v1/experiments/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    body,
    headers,
  });
}

export function startExperiment(id: string): Promise<MessageResponse> {
  return request<MessageResponse>(
    `/admin/v1/experiments/${encodeURIComponent(id)}/start`,
    { method: 'POST' },
  );
}

export function stopExperiment(id: string): Promise<MessageResponse> {
  return request<MessageResponse>(
    `/admin/v1/experiments/${encodeURIComponent(id)}/stop`,
    { method: 'POST' },
  );
}

export function deleteExperiment(id: string): Promise<MessageResponse> {
  return request<MessageResponse>(`/admin/v1/experiments/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  });
}
