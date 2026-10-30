import typography from "@tailwindcss/typography";

const v = (name) => `rgb(var(--${name}) / <alpha-value>)`;

/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{vue,ts,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        bg: v("bg"),
        surface: v("surface"),
        elevated: v("elevated"),
        line: v("border"),
        fg: v("fg"),
        muted: v("muted"),
        subtle: v("subtle"),
        accent: v("accent"),
        "accent-fg": v("accent-fg"),
        ok: v("ok"),
        warn: v("warn"),
        danger: v("danger"),
        info: v("info"),
      },
      borderColor: {
        DEFAULT: v("border"),
      },
      fontFamily: {
        sans: [
          "ui-sans-serif",
          "-apple-system",
          "BlinkMacSystemFont",
          "Inter",
          "Segoe UI",
          "Roboto",
          "sans-serif",
        ],
        mono: [
          "ui-monospace",
          "SF Mono",
          "JetBrains Mono",
          "Menlo",
          "Consolas",
          "monospace",
        ],
      },
      fontSize: {
        "2xs": ["10.5px", "14px"],
      },
    },
  },
  plugins: [typography],
};
