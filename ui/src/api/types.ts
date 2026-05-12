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

export interface ApiKeyListResponse {
  items: ApiKeySummary[];
}

export interface CreateApiKeyResponse extends ApiKeySummary {
  key: string;
}

export type UserStatus = 'pending' | 'active';
export type UserRole = 'admin' | 'member';

export interface UserSummary {
  userId: string;
  email: string;
  name: string | null;
  pictureUrl: string | null;
  status: UserStatus;
  role: UserRole;
  createdAt: number;
}

export interface UserListResponse {
  items: UserSummary[];
}

export interface AuthUser {
  userId: string;
  email: string;
  name: string | null;
  pictureUrl: string | null;
  role: UserRole;
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

export type Granularity = 'hour' | 'day';

export interface VariantResult {
  variantKey: string;
  isControl: boolean;
  exposures: number;
  converters: number;
  totalConversions: number;
  totalValue: number;
  conversionRate: number | null;
  ci95: [number, number] | null;
  lift: number | null;
  pValue: number | null;
}

export interface SrmShare {
  variantKey: string;
  expected: number;
  actual: number;
}

export interface SrmResult {
  chiSquare: number;
  pValue: number;
  warning: boolean;
  expected: SrmShare[];
}

export interface TimeSeriesBucket {
  bucketStartMs: number;
  perVariant: Record<string, number>;
}

export interface ResultsResponse {
  experimentId: string;
  experimentKey: string;
  metricName: string;
  windowStartMs: number;
  windowEndMs: number;
  granularity: Granularity;
  variants: VariantResult[];
  srm: SrmResult | null;
  timeSeries: TimeSeriesBucket[];
}

export interface ResultsQueryParams {
  start?: number;
  end?: number;
  granularity?: Granularity;
  metric?: string;
}
