import js from "@eslint/js";
import perfectionist from "eslint-plugin-perfectionist";
import reactHooks from "eslint-plugin-react-hooks";
import globals from "globals";
import tseslint from "typescript-eslint";

export default tseslint.config(
  {
    ignores: ["dist/**", "src-tauri/**", ".agents/**", "src/components/ui/**"],
  },
  js.configs.recommended,
  tseslint.configs.recommended,
  perfectionist.configs["recommended-natural"],
  reactHooks.configs.flat["recommended-latest"],
  {
    languageOptions: {
      globals: globals.browser,
    },
    rules: {
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", caughtErrorsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
    },
  },
);
