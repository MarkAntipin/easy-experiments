# Easy Experiments — landing page

One-screen marketing page for
[easy-experiments](https://github.com/MarkAntipin/easy-experiments). Astro +
Tailwind, brand-matched to the admin UI in `../ui/`.

## Run it

```bash
cd landing
npm install
npm run dev      # http://localhost:4321
```

```bash
npm run build    # static output in ./dist (includes sitemap-index.xml)
npm run preview  # serve the built output
```

`dist/` is fully static — drop it on any CDN.

## Before you deploy

1. **Set your real production URL** in two places (keep them in sync):
   - `src/config.ts` → `siteUrl`
   - `astro.config.mjs` → `site`
   - `public/robots.txt` → the `Sitemap:` line
2. **Drop a real Open Graph image** at `public/og-image.png` — 1200×630 PNG,
   under 1 MB. The page references it for Twitter, Slack, iMessage, etc. Until
   you do, link previews will be broken. Keep important content in the centre
   80 % "safe zone" — some platforms crop the edges.
3. **Set the admin / GitHub URLs** in `src/config.ts`.

## SEO

Out of the box:
- `<link rel="canonical">` is computed per page from `Astro.url`.
- Open Graph + Twitter card meta with width/height hints.
- `SoftwareApplication` JSON-LD in the document head so Google + AI assistants
  understand the product.
- `@astrojs/sitemap` emits `/sitemap-index.xml` + `/sitemap-0.xml` on build.
- `public/robots.txt` allows all crawlers and points at the sitemap.
- `index,follow,max-image-preview:large` meta robots.
- Apple touch icon, theme-color (light + dark), site name, locale.

After you deploy, validate previews with the
[Twitter card validator](https://cards-dev.twitter.com/validator), the
[Facebook sharing debugger](https://developers.facebook.com/tools/debug/), and
[Google's Rich Results test](https://search.google.com/test/rich-results).

## Mobile

- Body uses `min-h-dvh` (correct viewport height on iOS Safari).
- `viewport-fit=cover` + `env(safe-area-inset-*)` padding so content respects
  notch / home indicator.
- Headline scales `text-4xl → text-6xl → text-7xl` across mobile / tablet /
  desktop with `text-balance` to avoid awkward line breaks.
- Touch targets are 56 px tall (well above the 44 px minimum).
- `-webkit-tap-highlight-color: transparent`, `-webkit-text-size-adjust: 100%`,
  and input `font-size: 16px` (prevents iOS auto-zoom on focus).
- `prefers-reduced-motion` disables transitions for users who've asked for it.

## Where to edit things

| Want to change…              | File                              |
| ---------------------------- | --------------------------------- |
| Site URL, admin URL, etc.    | `src/config.ts`                   |
| Headline + CTAs              | `src/components/Hero.astro`       |
| Footer                       | `src/components/Footer.astro`     |
| `<head>` meta + JSON-LD      | `src/layouts/Base.astro`          |
| Brand colors                 | `tailwind.config.mjs`             |
| Global / mobile CSS          | `src/styles/global.css`           |
| Sitemap / robots             | `astro.config.mjs`, `public/robots.txt` |

Brand assets (`logo-*.png`, `favicon.svg`) are copied from `../ui/public/` so
the landing page and admin panel stay visually consistent. Re-copy them if
they change.
