import { Link } from 'react-router-dom';
import {
  BarChart3,
  Beaker,
  BookOpen,
  Code2,
  KeyRound,
  Lightbulb,
  ListChecks,
  Sparkles,
} from 'lucide-react';
import { PageBody, PageHeader } from '@/components/PageHeader';

const evaluateExample = `const res = await fetch(
  'https://your-easy-experiments.example.com/api/v1/experiments/evaluate',
  {
    method: 'POST',
    headers: {
      'X-Api-Key': process.env.EASY_EXPERIMENTS_KEY,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      experimentKey: 'checkout_button_copy',
      entityId: currentUser.id,        // or a stable anonymous cookie ID
      properties: { country: 'US' },   // optional, used by segment rules
    }),
  },
);

const { variantKey, config } = await res.json();
// config is { label: "Buy now" } or { label: "Get started" }
// If the experiment isn't running, variantKey is null. Fall back.

renderButton(config?.label ?? 'Buy now');`;

const trackExample = `await fetch('https://your-easy-experiments.example.com/api/v1/track', {
  method: 'POST',
  headers: {
    'X-Api-Key': process.env.EASY_EXPERIMENTS_KEY,
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    events: [{
      entityId: currentUser.id,    // same ID you used in /evaluate
      metricName: 'purchase',      // matches the primary metric you set
      value: 49.99,                // optional — for revenue-style metrics
      idempotencyKey: orderId,     // optional — prevents double-counting
    }],
  }),
});`;

const variantConfigExample = `// Variant "control"
{ "label": "Buy now" }

// Variant "treatment"
{ "label": "Get started" }`;

const toc = [
  { id: 'what', label: '1. What is an experiment?' },
  { id: 'overview', label: '2. The whole flow in a minute' },
  { id: 'walkthrough', label: '3. Walkthrough — a real example' },
  { id: 'glossary', label: '4. Glossary' },
  { id: 'tips', label: '5. Tips for trustworthy results' },
  { id: 'patterns', label: '6. Common patterns' },
];

function SectionHeading({
  id,
  icon: Icon,
  children,
}: {
  id: string;
  icon: typeof Beaker;
  children: React.ReactNode;
}) {
  return (
    <h2
      id={id}
      className="mt-12 flex scroll-mt-24 items-center gap-2.5 text-xl font-semibold text-slate-900"
    >
      <Icon aria-hidden className="h-5 w-5 text-brand-600" />
      {children}
    </h2>
  );
}

function StepHeading({
  n,
  children,
}: {
  n: number;
  children: React.ReactNode;
}) {
  return (
    <h3 className="mt-8 flex items-baseline gap-3 text-lg font-semibold text-slate-900">
      <span
        aria-hidden
        className="grid h-7 w-7 place-items-center rounded-full bg-brand-50 text-sm font-semibold text-brand-700"
      >
        {n}
      </span>
      {children}
    </h3>
  );
}

function P({ children }: { children: React.ReactNode }) {
  return (
    <p className="mt-3 text-base leading-relaxed text-slate-700">{children}</p>
  );
}

function Code({ children }: { children: React.ReactNode }) {
  return (
    <code className="rounded bg-slate-100 px-1.5 py-0.5 font-mono text-[0.9em] text-slate-800">
      {children}
    </code>
  );
}

function CodeBlock({
  language = 'js',
  children,
}: {
  language?: string;
  children: string;
}) {
  return (
    <div className="mt-4 overflow-hidden rounded-lg border border-slate-200 bg-slate-900">
      <div className="flex items-center justify-between border-b border-slate-700/60 px-4 py-2">
        <span className="text-xs font-medium uppercase tracking-wide text-slate-400">
          {language}
        </span>
      </div>
      <pre className="overflow-x-auto px-4 py-4 text-sm leading-relaxed text-slate-100">
        <code>{children}</code>
      </pre>
    </div>
  );
}

function Callout({
  tone = 'info',
  title,
  children,
}: {
  tone?: 'info' | 'warn' | 'tip';
  title: string;
  children: React.ReactNode;
}) {
  const toneClasses =
    tone === 'warn'
      ? 'border-amber-200 bg-amber-50 text-amber-900'
      : tone === 'tip'
        ? 'border-emerald-200 bg-emerald-50 text-emerald-900'
        : 'border-brand-200 bg-brand-50 text-brand-900';
  return (
    <div className={`mt-5 rounded-lg border px-4 py-3 ${toneClasses}`}>
      <div className="text-sm font-semibold">{title}</div>
      <div className="mt-1 text-sm leading-relaxed">{children}</div>
    </div>
  );
}

export function GuidePage() {
  return (
    <>
      <PageHeader
        title="How to launch an experiment"
        description="A plain-English walkthrough — from zero to your first running A/B test."
      />
      <PageBody>
        <div className="grid gap-10 lg:grid-cols-[1fr_240px]">
          {/* Main content */}
          <article className="max-w-3xl">
            <p className="text-lg leading-relaxed text-slate-600">
              You have a hunch — a new button, a new line of copy, a new flow —
              that you think will work better than what you ship today.{' '}
              <span className="font-semibold text-brand-700">
                EasyExperiments
              </span>{' '}
              shows the change to a slice of your users, watches what they do,
              and tells you which version won.
            </p>

            {/* 1. What */}
            <SectionHeading id="what" icon={Sparkles}>
              What is an experiment?
            </SectionHeading>
            <P>
              An experiment is a comparison. You have a current behavior — call
              it <Code>control</Code> — and a change you want to try — call it{' '}
              <Code>treatment</Code>. EasyExperiments randomly splits your
              users between them, records what each user did, and computes
              which version produced more of the outcome you care about.
            </P>
            <P>
              <strong>Real-life example.</strong> Your checkout button today
              says <Code>"Buy now"</Code>. Someone on the team thinks{' '}
              <Code>"Get started"</Code> will convert better. You don't know who
              is right. So you run an experiment: half the visitors see{' '}
              <Code>"Buy now"</Code>, half see <Code>"Get started"</Code>, and
              after a couple of weeks you compare purchases.
            </P>

            {/* 2. Overview */}
            <SectionHeading id="overview" icon={ListChecks}>
              The whole flow in a minute
            </SectionHeading>
            <ol className="mt-4 space-y-3 text-base leading-relaxed text-slate-700">
              <li>
                <strong>1. Create an API key</strong> so your code can talk to
                EasyExperiments.
              </li>
              <li>
                <strong>2. Define your experiment</strong> in this dashboard:
                what you're comparing (variants), who sees it (segments), and
                what counts as success (primary metric).
              </li>
              <li>
                <strong>3. Ask for the variant</strong> at the moment a user
                hits the change in your app — call <Code>POST /evaluate</Code>{' '}
                and render whatever it returns.
              </li>
              <li>
                <strong>4. Track conversions</strong> when the user does the
                thing you measure — call <Code>POST /track</Code>.
              </li>
              <li>
                <strong>5. Read the results</strong> on the experiment's
                Results page when enough users have come through.
              </li>
            </ol>

            {/* 3. Walkthrough */}
            <SectionHeading id="walkthrough" icon={Beaker}>
              Walkthrough — testing a checkout button
            </SectionHeading>
            <P>
              We'll launch one experiment end to end:{' '}
              <Code>"Buy now"</Code> vs <Code>"Get started"</Code> on the
              checkout button of an online store. Same example all the way
              through.
            </P>

            <StepHeading n={1}>Create an API key</StepHeading>
            <P>
              Go to{' '}
              <Link
                to="/api-keys"
                className="font-medium text-brand-700 underline-offset-2 hover:underline"
              >
                API Keys
              </Link>{' '}
              and click <strong>New key</strong>. Give it a name like{' '}
              <Code>production-web</Code>. Copy the key it shows you — that's
              the only time you'll see it. Your code sends it as the{' '}
              <Code>X-Api-Key</Code> header on every request.
            </P>
            <Callout tone="warn" title="Treat keys like passwords">
              An API key gives anyone holding it the ability to assign variants
              and report metrics for your company. Don't paste them into
              client-side code you can't trust. For browser apps, route
              EasyExperiments calls through your own backend.
            </Callout>

            <StepHeading n={2}>Define the experiment</StepHeading>
            <P>
              Click{' '}
              <Link
                to="/experiments/new"
                className="font-medium text-brand-700 underline-offset-2 hover:underline"
              >
                Experiments → New
              </Link>{' '}
              and fill in the form.
            </P>
            <div className="mt-4 overflow-hidden rounded-lg border border-slate-200 bg-white">
              <table className="min-w-full divide-y divide-slate-200 text-base">
                <thead className="bg-slate-50 text-left text-sm uppercase tracking-wide text-slate-500">
                  <tr>
                    <th className="px-5 py-3 font-medium">Field</th>
                    <th className="px-5 py-3 font-medium">What to put</th>
                    <th className="px-5 py-3 font-medium">In our example</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100 text-slate-700">
                  <tr>
                    <td className="px-5 py-3 font-medium text-slate-900">
                      Key
                    </td>
                    <td className="px-5 py-3">
                      A short, lowercase, code-friendly name
                    </td>
                    <td className="px-5 py-3 font-mono text-sm">
                      checkout_button_copy
                    </td>
                  </tr>
                  <tr>
                    <td className="px-5 py-3 font-medium text-slate-900">
                      Description
                    </td>
                    <td className="px-5 py-3">
                      One sentence to remind future you why this exists
                    </td>
                    <td className="px-5 py-3">
                      Test "Get started" vs "Buy now" on the checkout page
                    </td>
                  </tr>
                  <tr>
                    <td className="px-5 py-3 font-medium text-slate-900">
                      Primary metric
                    </td>
                    <td className="px-5 py-3">
                      The one number you'll judge winners by
                    </td>
                    <td className="px-5 py-3 font-mono text-sm">purchase</td>
                  </tr>
                  <tr>
                    <td className="px-5 py-3 font-medium text-slate-900">
                      Variants
                    </td>
                    <td className="px-5 py-3">
                      The versions you're comparing
                    </td>
                    <td className="px-5 py-3">
                      <span className="font-mono text-sm">control</span> +{' '}
                      <span className="font-mono text-sm">treatment</span>
                    </td>
                  </tr>
                  <tr>
                    <td className="px-5 py-3 font-medium text-slate-900">
                      Segments
                    </td>
                    <td className="px-5 py-3">
                      Who's included and how traffic splits between variants
                    </td>
                    <td className="px-5 py-3">
                      One segment, 100% rollout, 50/50 split
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
            <P>
              The <Code>config</Code> field on each variant is a JSON object
              your code reads to render the difference. For our button:
            </P>
            <CodeBlock language="json">{variantConfigExample}</CodeBlock>
            <P>
              A <strong>segment</strong> answers two questions: <em>who</em>{' '}
              enters the experiment, and <em>how</em> the entered users get
              split. For your first experiment, leave one segment with no
              constraints, <Code>rolloutPercent: 100</Code>, and a 50/50 split
              between control and treatment. That means: every visitor is in
              the experiment; half see one button, half see the other.
            </P>
            <P>
              Hit <strong>Create</strong>, then <strong>Start</strong> to begin
              recording exposures.
            </P>

            <StepHeading n={3}>Show the variant in your code</StepHeading>
            <P>
              Wherever you render the checkout button, ask EasyExperiments
              which version this user should see:
            </P>
            <CodeBlock>{evaluateExample}</CodeBlock>
            <P>A few rules of the road:</P>
            <ul className="mt-3 list-disc space-y-2 pl-6 text-base leading-relaxed text-slate-700">
              <li>
                <strong>
                  <Code>entityId</Code> must be stable per user.
                </strong>{' '}
                The same user must always get the same variant — that's how the
                math works. Use your user ID for logged-in users; a cookie or
                device ID for anonymous ones. Don't switch midway.
              </li>
              <li>
                <strong>
                  <Code>properties</Code> are for segment targeting.
                </strong>{' '}
                If your segment has a constraint{' '}
                <Code>country = "US"</Code>, EasyExperiments looks at{' '}
                <Code>properties.country</Code> on the request.
              </li>
              <li>
                <strong>EasyExperiments handles bucketing.</strong> You don't
                decide who's in control vs treatment — the server does,
                deterministically, from <Code>entityId</Code>. Same user, same
                variant, every time.
              </li>
              <li>
                <strong>Calling on every render is fine.</strong> The server
                caches and dedups exposures (one exposure per user per hour).
                Don't try to outsmart it with your own caching.
              </li>
              <li>
                <strong>
                  Always have a fallback for <Code>variantKey === null</Code>.
                </strong>{' '}
                That's what you get if the experiment isn't running, or the
                user isn't in any segment. Render your current behavior.
              </li>
            </ul>

            <StepHeading n={4}>Track conversions</StepHeading>
            <P>
              When the user does the thing you measure — in our case, buying —
              tell EasyExperiments:
            </P>
            <CodeBlock>{trackExample}</CodeBlock>
            <P>
              <Code>metricName: 'purchase'</Code> matches the primary metric
              you set on the experiment. That's how EasyExperiments later
              connects "this user converted" with "this user was in variant X."
            </P>
            <Callout tone="tip" title="Track once, reuse everywhere">
              You can fire as many metric names as you want —{' '}
              <Code>signup</Code>, <Code>add_to_cart</Code>,{' '}
              <Code>refund</Code>, anything. They're not tied to specific
              experiments at write time. Any future experiment can pick any of
              your tracked metrics as its primary metric. Instrument once,
              measure forever.
            </Callout>
            <Callout tone="info" title="Use the same entityId in both calls">
              If <Code>/evaluate</Code> said{' '}
              <Code>entityId: "user_42"</Code>, then{' '}
              <Code>/track</Code> for that user must also send{' '}
              <Code>entityId: "user_42"</Code>. Otherwise EasyExperiments can't
              attribute the conversion to the variant they saw.
            </Callout>

            <StepHeading n={5}>Read the results</StepHeading>
            <P>
              Open the experiment and click <strong>Results</strong>. Per
              variant, you'll see:
            </P>
            <ul className="mt-3 list-disc space-y-2 pl-6 text-base leading-relaxed text-slate-700">
              <li>
                <strong>Exposures</strong> — how many users saw the variant.
              </li>
              <li>
                <strong>Converters / Conversion rate</strong> — what fraction
                of them did the thing you measured.
              </li>
              <li>
                <strong>Lift</strong> — how much better treatment is vs
                control (e.g. <Code>+12.4%</Code>).
              </li>
              <li>
                <strong>p-value</strong> — the chance the lift is just random
                noise. <Code>&lt; 0.05</Code> is the usual bar.
              </li>
              <li>
                <strong>CI95</strong> — a range you're 95% confident the true
                conversion rate falls in.
              </li>
              <li>
                <strong>SRM banner</strong> — lights up if the actual traffic
                split looks broken (e.g. 70/30 instead of 50/50). When this
                fires, treat any "winner" as suspect until you fix the cause.
              </li>
            </ul>

            {/* 4. Glossary */}
            <SectionHeading id="glossary" icon={BookOpen}>
              Glossary
            </SectionHeading>
            <dl className="mt-4 space-y-3 text-base leading-relaxed text-slate-700">
              <div>
                <dt className="font-semibold text-slate-900">Variant</dt>
                <dd>
                  A version you're testing. Always have one called{' '}
                  <Code>control</Code> — your current behavior.
                </dd>
              </div>
              <div>
                <dt className="font-semibold text-slate-900">Segment</dt>
                <dd>
                  A rule for <em>who</em> enters the experiment and how their
                  traffic is split between variants.
                </dd>
              </div>
              <div>
                <dt className="font-semibold text-slate-900">Primary metric</dt>
                <dd>
                  The single number you declare a winner by. Pick it before
                  starting; don't change it midway.
                </dd>
              </div>
              <div>
                <dt className="font-semibold text-slate-900">Entity ID</dt>
                <dd>
                  The stable identifier of the user you're testing on. Same ID
                  every time, in both <Code>/evaluate</Code> and{' '}
                  <Code>/track</Code>.
                </dd>
              </div>
              <div>
                <dt className="font-semibold text-slate-900">Exposure</dt>
                <dd>
                  One user being shown a variant. EasyExperiments counts at
                  most one exposure per user per hour, so calling repeatedly is
                  safe.
                </dd>
              </div>
              <div>
                <dt className="font-semibold text-slate-900">Conversion</dt>
                <dd>
                  The user did the thing you're measuring (a tracked event
                  matching the primary metric, after their first exposure).
                </dd>
              </div>
              <div>
                <dt className="font-semibold text-slate-900">Lift</dt>
                <dd>
                  Percentage improvement of a variant over control. Positive is
                  good.
                </dd>
              </div>
              <div>
                <dt className="font-semibold text-slate-900">p-value</dt>
                <dd>
                  Probability the result is just chance. Lower means more
                  confident. Common cutoff: <Code>0.05</Code>.
                </dd>
              </div>
              <div>
                <dt className="font-semibold text-slate-900">SRM</dt>
                <dd>
                  Sample Ratio Mismatch. A statistical alarm that the actual
                  split doesn't match the one you configured — usually a bug in
                  how your code calls <Code>/evaluate</Code>.
                </dd>
              </div>
            </dl>

            {/* 5. Tips */}
            <SectionHeading id="tips" icon={Lightbulb}>
              Tips for trustworthy results
            </SectionHeading>
            <ul className="mt-4 list-disc space-y-3 pl-6 text-base leading-relaxed text-slate-700">
              <li>
                <strong>Pick the metric before you start.</strong> Switching it
                midway because the original didn't move is data dredging.
              </li>
              <li>
                <strong>Let it run.</strong> A few thousand users per variant
                is a rough minimum for most consumer products. The smaller the
                expected lift, the longer you need.
              </li>
              <li>
                <strong>Don't peek and stop.</strong> Decide a runtime in
                advance and stick to it. "Significant" early results often
                unwind with more data.
              </li>
              <li>
                <strong>Watch the SRM banner.</strong> If the split is broken,
                no number on the page is trustworthy — fix the cause before
                drawing conclusions.
              </li>
              <li>
                <strong>Test one thing at a time.</strong> If treatment changes
                the button copy <em>and</em> its color, you won't know which
                won.
              </li>
              <li>
                <strong>Keep the entity ID consistent.</strong> Anonymous user
                visits, you assign a cookie ID, they sign up, you switch to
                user ID — that's two different "users" to EasyExperiments and
                your math gets messy. Pick one and stick to it.
              </li>
            </ul>

            {/* 6. Patterns */}
            <SectionHeading id="patterns" icon={Code2}>
              Common patterns
            </SectionHeading>
            <div className="mt-4 space-y-5">
              <div className="rounded-lg border border-slate-200 bg-white p-5">
                <h3 className="text-base font-semibold text-slate-900">
                  Feature flag (no testing)
                </h3>
                <p className="mt-2 text-base leading-relaxed text-slate-700">
                  One variant at 100%. Use <Code>/evaluate</Code> to gate the
                  feature on or off without comparing anything. Skip{' '}
                  <Code>/track</Code> and the Results page entirely.
                </p>
              </div>
              <div className="rounded-lg border border-slate-200 bg-white p-5">
                <h3 className="text-base font-semibold text-slate-900">
                  Gradual rollout
                </h3>
                <p className="mt-2 text-base leading-relaxed text-slate-700">
                  One segment with <Code>rolloutPercent: 5</Code>, gradually
                  bumped up over days. Users inside the rollout see treatment;
                  users outside get <Code>variantKey: null</Code> and your
                  fallback. Good for de-risking risky changes.
                </p>
              </div>
              <div className="rounded-lg border border-slate-200 bg-white p-5">
                <h3 className="text-base font-semibold text-slate-900">
                  Country / cohort targeting
                </h3>
                <p className="mt-2 text-base leading-relaxed text-slate-700">
                  Add a constraint to the segment, e.g.{' '}
                  <Code>country IN ["US", "CA"]</Code>. Only matching users
                  enter; everyone else is unaffected. Pass{' '}
                  <Code>properties: {`{ country: "..." }`}</Code> in your{' '}
                  <Code>/evaluate</Code> call.
                </p>
              </div>
              <div className="rounded-lg border border-slate-200 bg-white p-5">
                <h3 className="text-base font-semibold text-slate-900">
                  Multi-variant test
                </h3>
                <p className="mt-2 text-base leading-relaxed text-slate-700">
                  Need to compare three or more options? Add more variants and
                  split the percentages: <Code>control 34</Code>,{' '}
                  <Code>treatment_a 33</Code>, <Code>treatment_b 33</Code>.
                  EasyExperiments computes lift and p-value of each treatment
                  vs control.
                </p>
              </div>
            </div>

            <div className="mt-12 flex items-center justify-between rounded-lg border border-brand-200 bg-brand-50 px-5 py-4">
              <div>
                <div className="text-base font-semibold text-brand-900">
                  Ready to launch yours?
                </div>
                <div className="text-sm text-brand-800">
                  Grab a key, then create your first experiment.
                </div>
              </div>
              <div className="flex items-center gap-2">
                <Link
                  to="/api-keys"
                  className="inline-flex items-center gap-1.5 rounded-md border border-brand-300 bg-white px-3 py-2 text-sm font-medium text-brand-700 hover:bg-brand-100"
                >
                  <KeyRound aria-hidden className="h-4 w-4" />
                  API Keys
                </Link>
                <Link
                  to="/experiments/new"
                  className="inline-flex items-center gap-1.5 rounded-md bg-brand-600 px-3 py-2 text-sm font-medium text-white hover:bg-brand-700"
                >
                  <Beaker aria-hidden className="h-4 w-4" />
                  New experiment
                </Link>
              </div>
            </div>
          </article>

          {/* TOC */}
          <aside className="hidden lg:block">
            <div className="sticky top-6 rounded-lg border border-slate-200 bg-white p-4">
              <div className="mb-2 flex items-center gap-2 text-sm font-semibold text-slate-900">
                <BarChart3 aria-hidden className="h-4 w-4 text-brand-600" />
                On this page
              </div>
              <nav>
                <ul className="space-y-1.5 text-sm">
                  {toc.map((item) => (
                    <li key={item.id}>
                      <a
                        href={`#${item.id}`}
                        className="block rounded px-2 py-1 text-slate-600 hover:bg-slate-50 hover:text-slate-900"
                      >
                        {item.label}
                      </a>
                    </li>
                  ))}
                </ul>
              </nav>
            </div>
          </aside>
        </div>
      </PageBody>
    </>
  );
}
