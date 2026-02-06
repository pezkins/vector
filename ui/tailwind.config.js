/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./index.html",
    "./src/**/*.rs",
  ],
  theme: {
    extend: {
      colors: {
        // Theme-aware colors using CSS variables
        'theme': {
          'bg': 'var(--color-bg)',
          'surface': 'var(--color-surface)',
          'surface-hover': 'var(--color-surface-hover)',
          'border': 'var(--color-border)',
          'text': 'var(--color-text)',
          'text-secondary': 'var(--color-text-secondary)',
          'muted': 'var(--color-muted)',
        },
        // Accent colors (same in both themes)
        'accent': {
          'DEFAULT': 'var(--color-accent)',
          'hover': 'var(--color-accent-hover)',
        },
        'success': 'var(--color-success)',
        'warning': 'var(--color-warning)',
        'error': 'var(--color-error)',
        // Component type colors
        'source': 'var(--color-source)',
        'transform': 'var(--color-transform)',
        'sink': 'var(--color-sink)',
        // Legacy vectorize namespace (for backward compatibility)
        'vectorize': {
          'bg': 'var(--color-bg)',
          'surface': 'var(--color-surface)',
          'border': 'var(--color-border)',
          'text': 'var(--color-text)',
          'muted': 'var(--color-text-secondary)',
          'accent': 'var(--color-accent)',
          'success': 'var(--color-success)',
          'warning': 'var(--color-warning)',
          'error': 'var(--color-error)',
          'source': 'var(--color-source)',
          'transform': 'var(--color-transform)',
          'sink': 'var(--color-sink)',
        }
      },
      backgroundColor: {
        'theme': {
          'bg': 'var(--color-bg)',
          'surface': 'var(--color-surface)',
          'surface-hover': 'var(--color-surface-hover)',
        },
      },
      borderColor: {
        'theme': {
          'DEFAULT': 'var(--color-border)',
          'border': 'var(--color-border)',
        },
      },
      textColor: {
        'theme': {
          'DEFAULT': 'var(--color-text)',
          'text': 'var(--color-text)',
          'secondary': 'var(--color-text-secondary)',
          'muted': 'var(--color-muted)',
        },
      },
      ringOffsetColor: {
        'theme': 'var(--color-bg)',
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
