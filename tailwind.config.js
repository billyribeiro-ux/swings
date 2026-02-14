/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    extend: {
      colors: {
        navy: '#0B1D3A',
        'navy-mid': '#132B50',
        'deep-blue': '#1A3A6B',
        teal: '#0FA4AF',
        'teal-light': '#15C5D1',
        'teal-glow': 'rgba(15, 164, 175, 0.15)',
        gold: '#D4A843',
        'gold-light': '#E8C76A',
        'off-white': '#F7F8FA',
        'grey-100': '#EEF0F4',
        'grey-200': '#D8DCE4',
        'grey-400': '#8B95A8',
        'grey-600': '#5A6478',
        'grey-800': '#2E3749',
        red: '#E04848',
        green: '#22B573',
      },
      fontFamily: {
        heading: ['var(--font-heading)'],
        ui: ['var(--font-ui)'],
      },
    },
  },
  plugins: [],
};
