/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        brand: {
          DEFAULT: "#0f172a",   // slate-900
          accent: "#2563eb",    // blue-600
        },
      },
    },
  },
  plugins: [],
};
