/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        ink: "#17201b",
        moss: "#315b43",
        paper: "#f7f5ef",
        line: "#d9ded5",
        amber: "#d6892b",
      },
    },
  },
  plugins: [],
};

