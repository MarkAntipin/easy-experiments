/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        // Vivid blue from the "Easy" wordmark and top of the flask gradient.
        brand: {
          50: '#eef3ff',
          100: '#dde6ff',
          200: '#bccfff',
          300: '#94afff',
          400: '#6688ff',
          500: '#4365ff',
          600: '#2a4eff',
          700: '#1d3ce6',
          800: '#1c33b4',
          900: '#1d2f8a',
          950: '#131a4e',
        },
        // Deep navy used for the flask outline and "Experiments" wordmark.
        ink: {
          50: '#f5f6fa',
          100: '#e9ebf3',
          200: '#cdd2e3',
          300: '#a4abca',
          400: '#717cab',
          500: '#4f5b91',
          600: '#3a4378',
          700: '#2c3360',
          800: '#1d2244',
          900: '#131a3d',
          950: '#0d122e',
        },
        // Purple from the bottom of the flask liquid gradient.
        accent: {
          50: '#f4ebff',
          100: '#e9d6ff',
          200: '#d2adff',
          300: '#b87fff',
          400: '#9650ff',
          500: '#7c2bef',
          600: '#6e26d9',
          700: '#5e1eb6',
          800: '#4f1c92',
          900: '#421b76',
          950: '#2a1052',
        },
      },
      backgroundImage: {
        // Matches the flask's blue → purple gradient.
        'brand-gradient': 'linear-gradient(135deg, #2a4eff 0%, #6e26d9 100%)',
      },
      boxShadow: {
        'brand-glow': '0 10px 30px -12px rgba(42, 78, 255, 0.45)',
      },
      fontFamily: {
        sans: [
          'ui-sans-serif',
          'system-ui',
          '-apple-system',
          'Segoe UI',
          'Roboto',
          'Helvetica',
          'Arial',
          'sans-serif',
        ],
        mono: [
          'ui-monospace',
          'SFMono-Regular',
          'Menlo',
          'Monaco',
          'Consolas',
          'monospace',
        ],
      },
    },
  },
  plugins: [],
};
