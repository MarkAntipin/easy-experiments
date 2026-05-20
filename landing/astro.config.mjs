import { defineConfig } from 'astro/config';
import tailwind from '@astrojs/tailwind';
import sitemap from '@astrojs/sitemap';

// Used for canonical URLs, the OG `og:url`, sitemap, etc.
// Change this to your real production URL before deploying.
export default defineConfig({
  site: 'https://easy-experiments.com',
  trailingSlash: 'never',
  integrations: [tailwind({ applyBaseStyles: false }), sitemap()],
});
