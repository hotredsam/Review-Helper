import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// Vitest runs the React component/store tests in a jsdom environment. It does
// not need the Tailwind plugin (tests assert behavior, not styling), and CSS
// imports are skipped.
export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    css: false,
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
