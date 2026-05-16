# Easy Experiments

A/B testing tool that runs on a $5 VPS

Easy Experiments allows companies to run A/B tests and feature experiments on their own infrastructure, without paying thousands of dollars for third-party experimentation platforms.

It helps teams test ideas, compare product changes, track results, and make better decisions while keeping costs low and data under their control.

## Quickstart

```bash
docker build -t easy-experiments .

docker run -d -p 18200:18200 \
  -e JWT_SECRET=secret \
  -e ADMIN_EMAIL=you@example.com \
  -e ADMIN_PASSWORD=password \
  -v ee-data:/data \
  --name easy-experiments \
  easy-experiments
```

Open <http://localhost:18200>, sign in, create your first experiment, then
create an API key from the **API Keys** screen — the plaintext is shown only
once, so copy it now.

## Use the SDK API

All SDK endpoints take an `X-Api-Key: eek-...` header and use camelCase JSON.

### Evaluate an experiment

```bash
curl -X POST http://localhost:18200/api/v1/experiments/evaluate \
  -H 'Content-Type: application/json' \
  -H 'X-Api-Key: eek-...' \
  -d '{
        "experimentKey": "new-checkout",
        "entityId": "user-42",
        "properties": { "country": "US", "plan": "pro" }
      }'
```

```json
{ "experimentKey": "new-checkout", "variantKey": "treatment", "config": { "buttonColor": "green" } }
```

`variantKey` is `null` when the entity doesn't match any segment or the
experiment isn't running. Assignment is deterministic per `(experiment, entity)`
and exposures are deduped server-side, so it's safe to call on every request.

### Track a metric

```bash
curl -X POST http://localhost:18200/api/v1/track \
  -H 'Content-Type: application/json' \
  -H 'X-Api-Key: eek-...' \
  -d '{
        "events": [
          { "entityId": "user-42", "metricName": "checkout_completed", "value": 49.99, "idempotencyKey": "order-1234" }
        ]
      }'
```
