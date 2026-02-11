/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        apple: {
          bg: '#F5F5F7',
          card: '#FFFFFF',
          text: '#1D1D1F',
          secondary: '#86868B',
          accent: '#0066CC',
          border: 'rgba(0, 0, 0, 0.1)',
        }
      },
      fontFamily: {
        sans: [
          'SF Pro Text',
          'Inter',
          '-apple-system',
          'BlinkMacSystemFont',
          'Segoe UI',
          'Roboto',
          'Helvetica Neue',
          'Arial',
          'sans-serif'
        ],
      },
      borderWidth: {
        '0.5': '0.5px',
      }
    },
  },
  plugins: [],
}
