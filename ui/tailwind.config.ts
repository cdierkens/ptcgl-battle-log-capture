import type { Config } from "tailwindcss";
import { baseConfig } from "@pokedojo/config/tailwind.config";
import reactAria from "tailwindcss-react-aria-components";

const config: Config = {
  ...baseConfig,
  content: [
    "./index.html",
    "./src/**/*.{ts,tsx}",
    "../../../packages/ui/src/**/*.{ts,tsx}",
  ],
  plugins: [reactAria],
};

export default config;