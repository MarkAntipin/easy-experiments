# k6 Load Test

## Run

```bash
export BASE_URL=https://app.easy-experiments.dev
export EXPERIMENT_KEY=k6-load-test
export API_KEY='<your eek-... key>'

PROFILE=smoke k6 run loadtest/script.js
PROFILE=proof k6 run loadtest/script.js
PROFILE=spike k6 run loadtest/script.js
PROFILE=stress k6 run loadtest/script.js
```

## Profiles

- `smoke`: 10 requests/second for a short validation run.
- `proof`: ramps to 500 requests/second and holds for 30 minutes.
- `spike`: proves recovery from a short 1,500 requests/second burst.
- `stress`: climbs from 250 to 5,000 requests/second to find the ceiling.

The default profile is `smoke`.

## Traffic Shape

Default mix:

- 85% evaluate requests
- 15% track requests
- 10 metric events per track request
- 80% repeated entities from a 100k-user pool
- 20% unique entities

The evaluate payload matches the realistic experiment constraints:

```json
{
  "country": "US|GB|DE|FR|ES",
  "plan": "pro|business",
  "loggedIn": true,
  "age": 18,
  "device": "desktop|mobile|tablet"
}
```
