import eslint from "@eslint/js";
import tseslint from "typescript-eslint";
import prettier from "eslint-config-prettier";

export default tseslint.config(
  {
    ignores: [
      "**/dist/**",
      "**/node_modules/**",
      "**/coverage/**",
      "**/.turbo/**",
      "shims/**",
      "eslint.config.js",
    ],
  },
  eslint.configs.recommended,
  ...tseslint.configs.strictTypeChecked,
  {
    languageOptions: {
      parserOptions: {
        projectService: {
          allowDefaultProject: [
            "*.config.ts",
            "packages/*/vitest.config.ts",
            "apps/*/vite.config.ts",
          ],
        },
        tsconfigRootDir: import.meta.dirname,
      },
    },
    rules: {
      // Disabled: these fire on every usage of dependencies whose types
      // cannot be resolved by the project-service (supabase-js, MCP SDK,
      // zod, etc.).  They produce thousands of false-positives and add no
      // value until those libraries ship compatible type declarations.
      "@typescript-eslint/no-unsafe-assignment": "off",
      "@typescript-eslint/no-unsafe-call": "off",
      "@typescript-eslint/no-unsafe-member-access": "off",
      "@typescript-eslint/no-unsafe-return": "off",
      "@typescript-eslint/no-unsafe-argument": "off",

      // Template literals commonly interpolate non-string values (errors,
      // numbers, etc.) throughout the codebase – this is intentional.
      "@typescript-eslint/restrict-template-expressions": "off",

      // Fires on union members that become redundant only because a
      // dependency's types are unresolved (resolved as `any`).
      "@typescript-eslint/no-redundant-type-constituents": "off",
    },
  },
  // Config files parsed via allowDefaultProject lack strictNullChecks,
  // so rules that depend on it must be disabled for those files.
  {
    files: ["**/*.config.ts"],
    rules: {
      "@typescript-eslint/no-unnecessary-boolean-literal-compare": "off",
      "@typescript-eslint/no-unnecessary-condition": "off",
      "@typescript-eslint/no-useless-default-assignment": "off",
    },
  },
  prettier,
);
