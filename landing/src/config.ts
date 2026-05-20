// Edit these to point at your hosted instance and repo.
export const siteConfig = {
  name: 'Easy Experiments',
  tagline: 'Self-hosted A/B testing & feature flags on a $5 VPS',
  description:
    'Open source, self-hosted A/B testing and feature flags. A lightweight LaunchDarkly, Amplitude, and Optimizely alternative that runs on a $5 VPS in a single Docker container.',
  // Keep in sync with `site` in astro.config.mjs.
  siteUrl: 'https://easy-experiments.com',
  adminUrl: 'https://app.easy-experiments.com',
  githubUrl: 'https://github.com/MarkAntipin/easy-experiments',
  docsUrl: 'https://github.com/MarkAntipin/easy-experiments#readme',
  contactEmail: 'mark@luzia.com',
  // Drop a 1200x630 PNG at /public/og-image.png before deploying.
  ogImagePath: '/og-image.png',
  author: {
    name: 'Mark Antipin',
    url: 'https://github.com/MarkAntipin',
  },
};
