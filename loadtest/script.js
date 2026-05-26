import http from 'k6/http';
import { check } from 'k6';
import exec from 'k6/execution';

const BASE_URL = envString('BASE_URL', 'https://app.easy-experiments.dev').replace(/\/+$/, '');
const API_KEY = envString('API_KEY');
const EXPERIMENT_KEY = envString('EXPERIMENT_KEY', 'k6-load-test');
const PROFILE = envString('PROFILE', 'smoke').toLowerCase();

const EVALUATE_WEIGHT = envNumber('EVALUATE_WEIGHT', 85);
const TRACK_BATCH_SIZE = envInt('TRACK_BATCH_SIZE', 10);
const USER_POOL_SIZE = envInt('USER_POOL_SIZE', 100000);
const UNIQUE_ENTITY_RATE = envNumber('UNIQUE_ENTITY_RATE', 0.2);
const TRACK_IDEMPOTENCY = envBool('TRACK_IDEMPOTENCY', true);
const METRIC_NOISE_RATE = envNumber('METRIC_NOISE_RATE', 0.1);

const COUNTRIES = ['US', 'GB', 'DE', 'FR', 'ES'];
const PLANS = ['pro', 'business'];
const DEVICES = ['desktop', 'mobile', 'tablet'];
const METRICS = ['conversion_rate', 'checkout_started', 'checkout_completed'];

const HEADERS = {
  'Content-Type': 'application/json',
  'X-Api-Key': API_KEY,
};

export const options = buildOptions(PROFILE);

export function setup() {
  if (!API_KEY) {
    throw new Error('API_KEY is required. Use: API_KEY=... PROFILE=smoke k6 run loadtest/script.js');
  }

  const health = http.get(`${BASE_URL}/health`, {
    tags: { endpoint: 'health' },
  });
  if (health.status !== 200) {
    throw new Error(`Health check failed: GET ${BASE_URL}/health returned ${health.status}`);
  }

  const res = http.post(
    `${BASE_URL}/api/v1/experiments/evaluate`,
    JSON.stringify({
      experimentKey: EXPERIMENT_KEY,
      entityId: 'k6-setup-user',
      properties: matchingProperties(0),
    }),
    {
      headers: HEADERS,
      responseType: 'text',
      tags: { endpoint: 'evaluate', phase: 'setup' },
    },
  );

  if (res.status !== 200) {
    throw new Error(`Setup evaluate failed with status ${res.status}. Check API_KEY and BASE_URL.`);
  }

  const body = safeJson(res);
  if (!body || body.variantKey === null || body.variantKey === undefined) {
    throw new Error(
      `Setup evaluate returned no variant for experiment "${EXPERIMENT_KEY}". ` +
        'Start the experiment and verify the k6 properties match its constraints.',
    );
  }

  return { experimentKey: EXPERIMENT_KEY };
}

export default function (data) {
  const iteration = exec.scenario.iterationInTest;
  const entityId = pickEntityId(iteration);

  if (Math.random() * 100 < EVALUATE_WEIGHT) {
    evaluate(data.experimentKey, entityId, iteration);
  } else {
    track(entityId, iteration);
  }
}

function evaluate(experimentKey, entityId, iteration) {
  const res = http.post(
    `${BASE_URL}/api/v1/experiments/evaluate`,
    JSON.stringify({
      experimentKey,
      entityId,
      properties: matchingProperties(iteration),
    }),
    {
      headers: HEADERS,
      tags: { endpoint: 'evaluate' },
    },
  );

  check(res, {
    'evaluate 200': (r) => r.status === 200,
  }, { endpoint: 'evaluate' });
}

function track(entityId, iteration) {
  const events = [];
  for (let i = 0; i < TRACK_BATCH_SIZE; i += 1) {
    const metricName = Math.random() < METRIC_NOISE_RATE ? pick(METRICS) : 'conversion_rate';
    const event = {
      entityId: i === 0 ? entityId : stableEntityId(iteration + i),
      metricName,
      value: 1,
    };
    if (TRACK_IDEMPOTENCY) {
      event.idempotencyKey = `k6-${exec.scenario.name}-${exec.vu.idInTest}-${iteration}-${i}`;
    }
    events.push(event);
  }

  const res = http.post(
    `${BASE_URL}/api/v1/track`,
    JSON.stringify({ events }),
    {
      headers: HEADERS,
      tags: { endpoint: 'track' },
    },
  );

  check(res, {
    'track 200': (r) => r.status === 200,
  }, { endpoint: 'track' });
}

function matchingProperties(iteration) {
  return {
    country: COUNTRIES[iteration % COUNTRIES.length],
    plan: PLANS[iteration % PLANS.length],
    loggedIn: true,
    age: 18 + (iteration % 48),
    device: DEVICES[iteration % DEVICES.length],
  };
}

function pickEntityId(iteration) {
  if (Math.random() < UNIQUE_ENTITY_RATE) {
    return `k6-unique-${Date.now()}-${exec.vu.idInTest}-${iteration}`;
  }
  return stableEntityId(iteration);
}

function stableEntityId(iteration) {
  return `user-${iteration % USER_POOL_SIZE}`;
}

function pick(items) {
  return items[Math.floor(Math.random() * items.length)];
}

function safeJson(res) {
  try {
    return res.json();
  } catch (_) {
    return null;
  }
}

function buildOptions(profile) {
  const scenario = scenarioFor(profile);
  return {
    discardResponseBodies: true,
    scenarios: {
      mixed_data_plane: {
        executor: 'ramping-arrival-rate',
        timeUnit: '1s',
        exec: 'default',
        ...scenario,
      },
    },
    thresholds: {
      http_req_failed: [`rate<${envNumber('MAX_ERROR_RATE', profile === 'stress' ? 0.05 : 0.001)}`],
      'http_req_duration{endpoint:evaluate}': [
        `p(95)<${envInt('EVALUATE_P95_MS', 150)}`,
        `p(99)<${envInt('EVALUATE_P99_MS', 500)}`,
      ],
      'http_req_duration{endpoint:track}': [
        `p(95)<${envInt('TRACK_P95_MS', 250)}`,
        `p(99)<${envInt('TRACK_P99_MS', 750)}`,
      ],
    },
  };
}

function scenarioFor(profile) {
  switch (profile) {
    case 'smoke':
      return {
        preAllocatedVUs: envInt('PRE_ALLOCATED_VUS', 20),
        maxVUs: envInt('MAX_VUS', 100),
        stages: [
          { duration: '30s', target: 5 },
          { duration: '2m', target: 10 },
          { duration: '30s', target: 0 },
        ],
      };
    case 'proof':
      return {
        preAllocatedVUs: envInt('PRE_ALLOCATED_VUS', 300),
        maxVUs: envInt('MAX_VUS', 2000),
        stages: [
          { duration: '2m', target: 10 },
          { duration: '5m', target: 500 },
          { duration: '30m', target: 500 },
          { duration: '2m', target: 0 },
        ],
      };
    case 'spike':
      return {
        preAllocatedVUs: envInt('PRE_ALLOCATED_VUS', 500),
        maxVUs: envInt('MAX_VUS', 2500),
        stages: [
          { duration: '2m', target: 50 },
          { duration: '5m', target: 500 },
          { duration: '2m', target: 1500 },
          { duration: '5m', target: 500 },
          { duration: '2m', target: 0 },
        ],
      };
    case 'stress':
      return {
        preAllocatedVUs: envInt('PRE_ALLOCATED_VUS', 1500),
        maxVUs: envInt('MAX_VUS', 10000),
        stages: [
          { duration: '3m', target: 250 },
          { duration: '5m', target: 500 },
          { duration: '5m', target: 1000 },
          { duration: '5m', target: 2000 },
          { duration: '5m', target: 3000 },
          { duration: '5m', target: 4000 },
          { duration: '5m', target: 5000 },
          { duration: '2m', target: 0 },
        ],
      };
    default:
      throw new Error(`Unknown PROFILE "${profile}". Use smoke, proof, spike, or stress.`);
  }
}

function envString(name, fallback = '') {
  const value = __ENV[name];
  return value === undefined || value === '' ? fallback : value;
}

function envInt(name, fallback) {
  const raw = envString(name, String(fallback));
  const value = Number.parseInt(raw, 10);
  if (!Number.isFinite(value)) {
    throw new Error(`${name} must be an integer, got "${raw}"`);
  }
  return value;
}

function envNumber(name, fallback) {
  const raw = envString(name, String(fallback));
  const value = Number.parseFloat(raw);
  if (!Number.isFinite(value)) {
    throw new Error(`${name} must be a number, got "${raw}"`);
  }
  return value;
}

function envBool(name, fallback) {
  const raw = envString(name, String(fallback));
  return ['1', 'true', 'yes', 'on'].includes(raw.toLowerCase());
}
