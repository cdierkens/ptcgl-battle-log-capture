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
  {
    languageOptions: {
      globals: globals.browser,
    },
    // Classic hooks rules only; the React Compiler rules are a separate follow-up.
    plugins: { "react-hooks": reactHooks },
    rules: {
      "react-hooks/exhaustive-deps": "error",
      "react-hooks/rules-of-hooks": "error",
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", caughtErrorsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
    },
  },
);
