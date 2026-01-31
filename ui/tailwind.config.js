/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./index.html",
    "./src/**/*.rs",
  ],
  theme: {
    extend: {
      colors: {
        // Custom Vectorize color palette
        'vectorize': {
          'bg': '#0f172a',        // slate-900
          'surface': '#1e293b',   // slate-800
          'border': '#334155',    // slate-700
          'text': '#f8fafc',      // slate-50
          'muted': '#94a3b8',     // slate-400
          'accent': '#3b82f6',    // blue-500
          'success': '#22c55e',   // green-500
          'warning': '#f59e0b',   // amber-500
          'error': '#ef4444',     // red-500
          // Component type colors
          'source': '#8b5cf6',    // violet-500
          'transform': '#06b6d4', // cyan-500
          'sink': '#f97316',      // orange-500
        }
      },
      fontFamily: {
        'mono': ['JetBrains Mono', 'Fira Code', 'monospace'],
      },
      animation: {
        'fade-in': 'fadeIn 0.2s ease-out',
        'slide-up': 'slideUp 0.3s ease-out',
        'slide-down': 'slideDown 0.3s ease-out',
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        slideUp: {
          '0%': { transform: 'translateY(10px)', opacity: '0' },
          '100%': { transform: 'translateY(0)', opacity: '1' },
        },
        slideDown: {
          '0%': { transform: 'translateY(-10px)', opacity: '0' },
          '100%': { transform: 'translateY(0)', opacity: '1' },
        },
      },
    },
  },
  plugins: [],
}
