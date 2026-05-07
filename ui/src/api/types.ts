export type ExperimentStatus = 'draft' | 'running' | 'stopped' | 'deleted';

export type ConstraintOperator =
  | 'EQ'
  | 'NEQ'
  | 'GT'
  | 'GTE'
  | 'LT'
  | 'LTE'
  | 'IN'
  | 'NOT_IN';

export type ConstraintValue =
  | string
  | number
  | boolean
  | Array<string | number | boolean>;

export interface Constraint {
  property: string;
  operator: ConstraintOperator;
  value: ConstraintValue;
}

export interface Distribution {
  variantKey: string;
  percent: number;
}

export interface Segment {
  priority: number;
  rolloutPercent: number;
  constraints: Constraint[];
  distributions: Distribution[];
}

export interface Variant {
  key: string;
  isControl: boolean;
  config: Record<string, unknown>;
}

export interface ExperimentSummary {
  experimentId: string;
  key: string;
  description: string | null;
  status: ExperimentStatus;
  primaryMetric: string;
  startedAt: number | null;
  stoppedAt: number | null;
  createdAt: number;
  updatedAt: number;
}

export interface ExperimentDetail extends ExperimentSummary {
  variants: Variant[];
  segments: Segment[];
}

export interface ExperimentListResponse {
  items: ExperimentSummary[];
}

export interface CreateExperimentRequest {
  key: string;
  description?: string | null;
  primaryMetric: string;
  variants: Variant[];
  segments: Segment[];
}

export interface UpdateExperimentRequest {
  description?: string | null;
  primaryMetric?: string;
  variants?: Variant[];
  segments?: Segment[];
}

export interface CreateExperimentResponse {
  experimentId: string;
  message: string;
}

export interface MessageResponse {
  message: string;
}

export interface ApiKeySummary {
  apiKeyId: string;
  name: string;
  keyPrefix: string;
  createdAt: number;
}

export interface CreateApiKeyResponse extends ApiKeySummary {
  key: string;
}

export interface AuthUser {
  userId: string;
  email: string;
  name: string | null;
  pictureUrl: string | null;
}

export interface AuthCompany {
  companyId: string;
  name: string;
}

export interface LoginResponse {
  token: string;
  user: AuthUser;
  company: AuthCompany;
}

export interface ApiErrorBody {
  message: string;
}
