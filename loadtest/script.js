import http from 'k6/http';
import { check } from 'k6';
import { randomItem, randomIntBetween } from 'https://jslib.k6.io/k6-utils/1.4.0/index.js';

const API_KEY = __ENV.API_KEY;
const BASE_URL = __ENV.BASE_URL || 'http://127.0.0.1:18200';
const EXPERIMENT_KEYS = (__ENV.EXPERIMENT_KEYS || '').split(',').filter(Boolean);

if (!API_KEY) throw new Error('API_KEY env var is required (run cargo run --release --bin seed_loadtest first)');
if (EXPERIMENT_KEYS.length === 0) throw new Error('EXPERIMENT_KEYS env var is required');

const USER_POOL_SIZE = 10000;
const COUNTRIES = ['US', 'DE', 'FR', 'GB', 'JP', 'BR', 'IN', 'AU', 'CA', 'MX'];
const TIERS = [1, 2, 3];
const METRICS = ['conversion_rate', 'click_through', 'activation_rate', 'engagement'];

// Per-variant conversion probability — treatment wins by ~2.4x.
const CONVERSION_RATE = {
  control: 0.05,
  treatment: 0.12,
};

export const options = {
  discardResponseBodies: true,
  scenarios: {
    evaluate: {
      executor: 'constant-arrival-rate',
      rate: 5000,
      timeUnit: '1s',
      duration: '1h',
      preAllocatedVUs: 300,
      maxVUs: 1500,
      exec: 'evaluate',
    },
    funnel: {
      executor: 'constant-arrival-rate',
      rate: 200,
      timeUnit: '1s',
      duration: '1h',
      preAllocatedVUs: 40,
      maxVUs: 200,
      exec: 'funnel',
    },
  },
  thresholds: {
    'http_req_failed': ['rate<0.001'],
    'http_req_duration{scenario:evaluate}': ['p(95)<50', 'p(99)<150'],
    'http_req_duration{scenario:funnel}': ['p(95)<100', 'p(99)<250'],
  },
};

const headers = {
  'Content-Type': 'application/json',
  'X-Api-Key': API_KEY,
};

function entityId() {
  return `user-${randomIntBetween(1, USER_POOL_SIZE)}`;
}

export function evaluate() {
  const body = JSON.stringify({
    experimentKey: randomItem(EXPERIMENT_KEYS),
    entityId: entityId(),
    properties: {
      country: randomItem(COUNTRIES),
      tier: randomItem(TIERS),
    },
  });
  const res = http.post(`${BASE_URL}/api/v1/experiments/evaluate`, body, { headers });
  check(res, { 'evaluate 200': (r) => r.status === 200 });
}

export function funnel() {
  const eid = entityId();
  const expKey = randomItem(EXPERIMENT_KEYS);
  const evalRes = http.post(
    `${BASE_URL}/api/v1/experiments/evaluate`,
    JSON.stringify({
      experimentKey: expKey,
      entityId: eid,
      properties: { country: randomItem(COUNTRIES), tier: randomItem(TIERS) },
    }),
    { headers, responseType: 'text' },
  );
  if (!check(evalRes, { 'evaluate 200': (r) => r.status === 200 })) return;

  const variant = evalRes.json('variantKey');
  if (!variant) return;

  const convRate = CONVERSION_RATE[variant] ?? 0;
  if (Math.random() >= convRate) return;

  const n = randomIntBetween(1, 5);
  const events = [];
  for (let i = 0; i < n; i++) {
    events.push({
      entityId: eid,
      metricName: randomItem(METRICS),
      value: 1.0,
    });
  }
  const trackRes = http.post(`${BASE_URL}/api/v1/track`, JSON.stringify({ events }), { headers });
  check(trackRes, { 'track 200': (r) => r.status === 200 });
}
