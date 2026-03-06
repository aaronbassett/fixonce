# Testing, Linting, and CI Implementation Plan

**Goal:** Add comprehensive unit/integration tests for pure logic, strict linting/formatting, GitHub Actions CI on PRs, and Husky pre-push hooks — with zero 3rd-party API mocking.

**Architecture:** Vitest for tests (already a root devDep), ESLint 9 flat config + Prettier for code quality, Turbo orchestrates all tasks across the monorepo. Tests target only pure logic modules. Module-level singleton state (cache Map, listener Set) is isolated using `vi.resetModules()` + dynamic imports or `vi.useFakeTimers()`. The only acceptable mocks are `node:fs` (matching existing `config.test.ts` pattern) and `console.error` spies.

**Tech Stack:** Vitest, ESLint 9 (flat config), typescript-eslint, Prettier, eslint-config-prettier, Husky, GitHub Actions

---

### Task 1: Install Linting & Formatting Dependencies

**Files:**
- Modify: `package.json:11-16` (devDependencies)

**Step 1: Install dev dependencies at workspace root**

Run:
```bash
pnpm add -Dw eslint @eslint/js typescript-eslint prettier eslint-config-prettier
```

Expected: 5 new entries in root `package.json` devDependencies. Exit 0.

**Step 2: Verify installation**

Run:
```bash
pnpm exec eslint --version
pnpm exec prettier --version
```

Expected: ESLint v9.x, Prettier v3.x (exact versions may vary).

**Step 3: Commit**

```bash
git add package.json pnpm-lock.yaml
git commit -m "chore(deps): add eslint, prettier, and typescript-eslint"
```

---

### Task 2: Create ESLint Flat Config

**Files:**
- Create: `eslint.config.js`

**Step 1: Create the ESLint config**

Create `eslint.config.js` at the repo root:

```js
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
    ],
  },
  eslint.configs.recommended,
  ...tseslint.configs.strictTypeChecked,
  {
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
  },
  prettier,
);
```

**Step 2: Commit**

```bash
git add eslint.config.js
git commit -m "chore(lint): add eslint flat config with typescript-eslint strict"
```

---

### Task 3: Create Prettier Config

**Files:**
- Create: `.prettierrc`
- Create: `.prettierignore`

**Step 1: Create `.prettierrc`**

```json
{
  "trailingComma": "all"
}
```

Note: Existing code already uses double quotes, semicolons, and Prettier's default printWidth (80). Only `trailingComma` differs from Prettier defaults.

**Step 2: Create `.prettierignore`**

```
dist
node_modules
coverage
.turbo
pnpm-lock.yaml
```

**Step 3: Commit**

```bash
git add .prettierrc .prettierignore
git commit -m "chore(lint): add prettier config and ignore file"
```

---

### Task 4: Add Lint and Format Scripts

**Files:**
- Modify: `package.json:5-9` (root scripts)
- Modify: `packages/shared/package.json:13-16` (scripts)
- Modify: `packages/storage/package.json:13-15` (scripts)
- Modify: `packages/pipeline/package.json:13-15` (scripts)
- Modify: `packages/activity/package.json:13-15` (scripts)
- Modify: `apps/cli/package.json:8-12` (scripts)
- Modify: `apps/mcp-server/package.json:8-12` (scripts)
- Modify: `apps/web/package.json:14-18` (scripts)
- Modify: `apps/hooks/package.json:24-27` (scripts)

**Step 1: Add format scripts to root `package.json`**

Add to `"scripts"`:
```json
"format:check": "prettier --check .",
"format": "prettier --write ."
```

**Step 2: Add `"lint": "eslint ."` to every workspace package**

Add `"lint": "eslint ."` to the `"scripts"` section of each file:
- `packages/shared/package.json`
- `packages/storage/package.json`
- `packages/pipeline/package.json`
- `packages/activity/package.json`
- `apps/cli/package.json`
- `apps/mcp-server/package.json`
- `apps/web/package.json`
- `apps/hooks/package.json`

Do NOT add lint scripts to shims — they contain no TypeScript source.

**Step 3: Commit**

```bash
git add package.json packages/*/package.json apps/*/package.json
git commit -m "chore(lint): add lint and format scripts to all workspace packages"
```

---

### Task 5: Fix Lint Errors and Apply Formatting

**Step 1: Run the formatter**

Run:
```bash
pnpm format
```

Expected: Prettier reformats files to match config. Many files may change.

**Step 2: Run the linter**

Run:
```bash
pnpm lint
```

Expected: May produce errors. Fix any legitimate issues (unused vars, unsafe type assertions). If typescript-eslint strict rules produce false positives on patterns used throughout the codebase, add targeted rule overrides in `eslint.config.js`. Common ones to watch for:
- `@typescript-eslint/no-unused-vars` on type re-exports
- `@typescript-eslint/restrict-template-expressions` on error messages
- `@typescript-eslint/no-misused-promises` on fire-and-forget async calls

**Step 3: Iterate until both pass cleanly**

Run:
```bash
pnpm format:check && pnpm lint
```

Expected: Both exit 0 with zero errors.

**Step 4: Commit**

```bash
git add -A
git commit -m "style: apply prettier formatting and fix lint errors"
```

---

### Task 6: Unit Tests for FixOnceError (`packages/shared`)

**Files:**
- Create: `packages/shared/src/errors.test.ts`

**Context:** `packages/shared/src/errors.ts` exports `FixOnceError` (extends `Error`, has `stage`, `suggestion`, `toJSON()`) and 8 factory functions that create errors with preset `stage` values. All pure logic, no dependencies.

**Step 1: Write the test file**

Create `packages/shared/src/errors.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import {
  FixOnceError,
  validationError,
  storageError,
  qualityGateError,
  duplicateDetectionError,
  searchError,
  rewriteError,
  rerankError,
  embeddingError,
} from "./errors.js";

describe("FixOnceError", () => {
  it("is an instance of Error", () => {
    const err = new FixOnceError({
      stage: "test",
      reason: "something broke",
      suggestion: "fix it",
    });
    expect(err).toBeInstanceOf(Error);
    expect(err.name).toBe("FixOnceError");
  });

  it("exposes stage, message, and suggestion", () => {
    const err = new FixOnceError({
      stage: "validation",
      reason: "bad input",
      suggestion: "check your input",
    });
    expect(err.stage).toBe("validation");
    expect(err.message).toBe("bad input");
    expect(err.suggestion).toBe("check your input");
  });

  it("serializes to JSON with toJSON()", () => {
    const err = new FixOnceError({
      stage: "storage",
      reason: "write failed",
      suggestion: "retry",
    });
    expect(err.toJSON()).toEqual({
      stage: "storage",
      reason: "write failed",
      suggestion: "retry",
    });
  });
});

describe("error factory functions", () => {
  const factories = [
    { fn: validationError, stage: "validation" },
    { fn: storageError, stage: "storage" },
    { fn: qualityGateError, stage: "quality_gate" },
    { fn: duplicateDetectionError, stage: "duplicate_detection" },
    { fn: searchError, stage: "search" },
    { fn: rewriteError, stage: "rewrite" },
    { fn: rerankError, stage: "rerank" },
    { fn: embeddingError, stage: "embedding" },
  ] as const;

  for (const { fn, stage } of factories) {
    it(`${fn.name}() creates error with stage "${stage}"`, () => {
      const err = fn("reason", "suggestion");
      expect(err).toBeInstanceOf(FixOnceError);
      expect(err.stage).toBe(stage);
      expect(err.message).toBe("reason");
      expect(err.suggestion).toBe("suggestion");
    });
  }
});
```

**Step 2: Run to verify tests pass**

Run: `cd packages/shared && pnpm test`

Expected: PASS — 11 tests (3 for FixOnceError class + 8 for factories).

**Step 3: Commit**

```bash
git add packages/shared/src/errors.test.ts
git commit -m "test(shared): add unit tests for FixOnceError and factory functions"
```

---

### Task 7: Unit Tests for Zod Schemas (`packages/shared`)

**Files:**
- Create: `packages/shared/src/schema.test.ts`

**Context:** `packages/shared/src/schema.ts` exports Zod schemas for all input types. Tests validate parsing of valid inputs, rejection of invalid inputs, and default value application. All pure Zod — no mocks.

**Step 1: Write the test file**

Create `packages/shared/src/schema.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import {
  MemoryTypeSchema,
  SourceTypeSchema,
  CreatedByInputSchema,
  FeedbackTagSchema,
  SuggestedActionSchema,
  ComponentKeySchema,
  VersionPredicatesSchema,
  CreateMemoryInputSchema,
  QueryMemoriesInputSchema,
  SubmitFeedbackInputSchema,
  GetMemoryInputSchema,
  UpdateMemoryInputSchema,
} from "./schema.js";

describe("enum schemas", () => {
  it("MemoryTypeSchema accepts valid values", () => {
    expect(MemoryTypeSchema.parse("guidance")).toBe("guidance");
    expect(MemoryTypeSchema.parse("anti_pattern")).toBe("anti_pattern");
  });

  it("MemoryTypeSchema rejects invalid values", () => {
    expect(() => MemoryTypeSchema.parse("invalid")).toThrow();
  });

  it("SourceTypeSchema accepts valid values", () => {
    for (const v of ["correction", "discovery", "instruction"]) {
      expect(SourceTypeSchema.parse(v)).toBe(v);
    }
  });

  it("CreatedByInputSchema accepts ai and human only", () => {
    expect(CreatedByInputSchema.parse("ai")).toBe("ai");
    expect(CreatedByInputSchema.parse("human")).toBe("human");
    expect(() => CreatedByInputSchema.parse("human_modified")).toThrow();
  });

  it("FeedbackTagSchema accepts all 8 tags", () => {
    const tags = [
      "helpful", "not_helpful", "damaging", "accurate",
      "somewhat_accurate", "somewhat_inaccurate", "inaccurate", "outdated",
    ];
    for (const tag of tags) {
      expect(FeedbackTagSchema.parse(tag)).toBe(tag);
    }
  });

  it("SuggestedActionSchema accepts keep, remove, fix", () => {
    for (const v of ["keep", "remove", "fix"]) {
      expect(SuggestedActionSchema.parse(v)).toBe(v);
    }
  });

  it("ComponentKeySchema accepts all 12 component keys", () => {
    const keys = [
      "network", "node", "compact_compiler", "compact_runtime", "compact_js",
      "onchain_runtime", "ledger", "wallet_sdk", "midnight_js",
      "dapp_connector_api", "midnight_indexer", "proof_server",
    ];
    for (const k of keys) {
      expect(ComponentKeySchema.parse(k)).toBe(k);
    }
  });
});

describe("VersionPredicatesSchema", () => {
  it("accepts valid version predicates", () => {
    const result = VersionPredicatesSchema.parse({
      compact_compiler: ["0.14.0", "0.15.0"],
    });
    expect(result).toEqual({ compact_compiler: ["0.14.0", "0.15.0"] });
  });

  it("accepts null and undefined", () => {
    expect(VersionPredicatesSchema.parse(null)).toBeNull();
    expect(VersionPredicatesSchema.parse(undefined)).toBeUndefined();
  });

  it("rejects invalid component keys", () => {
    expect(() =>
      VersionPredicatesSchema.parse({ invalid_key: ["1.0.0"] }),
    ).toThrow();
  });
});

describe("CreateMemoryInputSchema", () => {
  const validInput = {
    title: "Test Memory",
    content: "This is the content of the memory.",
    summary: "A short summary.",
    memory_type: "guidance",
    source_type: "discovery",
    created_by: "human",
    language: "typescript",
    version_predicates: null,
  };

  it("parses valid input with defaults applied", () => {
    const result = CreateMemoryInputSchema.parse(validInput);
    expect(result.tags).toEqual([]);
    expect(result.confidence).toBe(0.5);
  });

  it("rejects empty title", () => {
    expect(() =>
      CreateMemoryInputSchema.parse({ ...validInput, title: "" }),
    ).toThrow();
  });

  it("rejects title over 500 chars", () => {
    expect(() =>
      CreateMemoryInputSchema.parse({ ...validInput, title: "x".repeat(501) }),
    ).toThrow();
  });

  it("rejects confidence out of range", () => {
    expect(() =>
      CreateMemoryInputSchema.parse({ ...validInput, confidence: 1.5 }),
    ).toThrow();
    expect(() =>
      CreateMemoryInputSchema.parse({ ...validInput, confidence: -0.1 }),
    ).toThrow();
  });

  it("accepts valid optional fields", () => {
    const result = CreateMemoryInputSchema.parse({
      ...validInput,
      tags: ["typescript", "react"],
      source_url: "https://example.com",
      confidence: 0.9,
      project_name: "fixonce",
    });
    expect(result.tags).toEqual(["typescript", "react"]);
    expect(result.source_url).toBe("https://example.com");
    expect(result.confidence).toBe(0.9);
  });

  it("rejects invalid source_url", () => {
    expect(() =>
      CreateMemoryInputSchema.parse({ ...validInput, source_url: "not-a-url" }),
    ).toThrow();
  });
});

describe("QueryMemoriesInputSchema", () => {
  it("applies defaults", () => {
    const result = QueryMemoriesInputSchema.parse({ query: "test" });
    expect(result.rewrite).toBe(true);
    expect(result.type).toBe("hybrid");
    expect(result.rerank).toBe(true);
    expect(result.max_results).toBe(5);
    expect(result.verbosity).toBe("small");
  });

  it("rejects empty query", () => {
    expect(() => QueryMemoriesInputSchema.parse({ query: "" })).toThrow();
  });

  it("rejects max_results out of range", () => {
    expect(() =>
      QueryMemoriesInputSchema.parse({ query: "test", max_results: 0 }),
    ).toThrow();
    expect(() =>
      QueryMemoriesInputSchema.parse({ query: "test", max_results: 51 }),
    ).toThrow();
  });
});

describe("SubmitFeedbackInputSchema", () => {
  it("requires valid UUID for memory_id", () => {
    expect(() =>
      SubmitFeedbackInputSchema.parse({ memory_id: "not-a-uuid" }),
    ).toThrow();
  });

  it("accepts minimal valid input", () => {
    const result = SubmitFeedbackInputSchema.parse({
      memory_id: "550e8400-e29b-41d4-a716-446655440000",
    });
    expect(result.tags).toEqual([]);
  });
});

describe("GetMemoryInputSchema", () => {
  it("defaults verbosity to large", () => {
    const result = GetMemoryInputSchema.parse({
      id: "550e8400-e29b-41d4-a716-446655440000",
    });
    expect(result.verbosity).toBe("large");
  });
});

describe("UpdateMemoryInputSchema", () => {
  it("allows partial updates", () => {
    const result = UpdateMemoryInputSchema.parse({
      id: "550e8400-e29b-41d4-a716-446655440000",
      title: "Updated Title",
    });
    expect(result.title).toBe("Updated Title");
    expect(result.content).toBeUndefined();
  });
});
```

**Step 2: Run to verify tests pass**

Run: `cd packages/shared && pnpm test`

Expected: PASS — all new schema tests + existing config tests.

**Step 3: Commit**

```bash
git add packages/shared/src/schema.test.ts
git commit -m "test(shared): add unit tests for zod schema validation"
```

---

### Task 8: Unit Tests for Version Filter (`packages/storage`)

**Files:**
- Create: `packages/storage/vitest.config.ts`
- Modify: `packages/storage/package.json:13-15` (add test script)
- Create: `packages/storage/src/version-filter.test.ts`

**Context:** `packages/storage/src/version-filter.ts` exports `filterByVersionPredicates()`. It's pure logic: takes a list of memories with `version_predicates` and a `DetectedVersions` record, returns the filtered list. Rules: null predicates = universal match, OR within component, AND across components. No external dependencies.

**Step 1: Create vitest config for storage package**

Create `packages/storage/vitest.config.ts`:

```ts
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    clearMocks: true,
  },
});
```

**Step 2: Add test script**

Add `"test": "vitest run"` to `packages/storage/package.json` scripts section.

**Step 3: Write the test file**

Create `packages/storage/src/version-filter.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { filterByVersionPredicates } from "./version-filter.js";

interface TestMemory {
  id: string;
  version_predicates: Record<string, string[]> | null;
}

function makeMemory(
  id: string,
  predicates: Record<string, string[]> | null,
): TestMemory {
  return { id, version_predicates: predicates };
}

describe("filterByVersionPredicates", () => {
  it("returns all memories when detectedVersions is empty", () => {
    const memories = [
      makeMemory("1", { compact_compiler: ["0.14.0"] }),
      makeMemory("2", { compact_compiler: ["0.15.0"] }),
    ];
    const result = filterByVersionPredicates(memories, {});
    expect(result).toHaveLength(2);
  });

  it("includes memories with null version_predicates (universal)", () => {
    const memories = [
      makeMemory("1", null),
      makeMemory("2", { compact_compiler: ["0.15.0"] }),
    ];
    const result = filterByVersionPredicates(memories, {
      compact_compiler: "0.14.0",
    });
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("1");
  });

  it("matches when detected version is in allowed list (OR within component)", () => {
    const memories = [
      makeMemory("1", { compact_compiler: ["0.14.0", "0.15.0"] }),
    ];
    const result = filterByVersionPredicates(memories, {
      compact_compiler: "0.15.0",
    });
    expect(result).toHaveLength(1);
  });

  it("excludes when detected version is not in allowed list", () => {
    const memories = [
      makeMemory("1", { compact_compiler: ["0.14.0"] }),
    ];
    const result = filterByVersionPredicates(memories, {
      compact_compiler: "0.16.0",
    });
    expect(result).toHaveLength(0);
  });

  it("requires all constrained components to match (AND across components)", () => {
    const memories = [
      makeMemory("1", {
        compact_compiler: ["0.14.0"],
        wallet_sdk: ["1.0.0"],
      }),
    ];
    expect(
      filterByVersionPredicates(memories, {
        compact_compiler: "0.14.0",
        wallet_sdk: "1.0.0",
      }),
    ).toHaveLength(1);
    expect(
      filterByVersionPredicates(memories, {
        compact_compiler: "0.14.0",
        wallet_sdk: "2.0.0",
      }),
    ).toHaveLength(0);
  });

  it("missing key in predicates means no constraint on that component", () => {
    const memories = [
      makeMemory("1", { compact_compiler: ["0.14.0"] }),
    ];
    const result = filterByVersionPredicates(memories, {
      wallet_sdk: "1.0.0",
    });
    expect(result).toHaveLength(1);
  });
});
```

**Step 4: Run to verify tests pass**

Run: `cd packages/storage && pnpm test`

Expected: PASS — 6 tests.

**Step 5: Commit**

```bash
git add packages/storage/vitest.config.ts packages/storage/package.json packages/storage/src/version-filter.test.ts
git commit -m "test(storage): add unit tests for version predicate filtering"
```

---

### Task 9: Unit Tests for Credential Check (`packages/pipeline`)

**Files:**
- Create: `packages/pipeline/vitest.config.ts`
- Modify: `packages/pipeline/package.json:13-15` (add test script)
- Create: `packages/pipeline/src/write/credential-check.test.ts`

**Context:** `packages/pipeline/src/write/credential-check.ts` exports `checkForCredentials(text)`. It tests 9 regex patterns against the input string and returns `{ found: boolean, patterns: string[] }`. Pure regex matching, no dependencies.

**Step 1: Create vitest config for pipeline package**

Create `packages/pipeline/vitest.config.ts`:

```ts
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    clearMocks: true,
  },
});
```

**Step 2: Add test script**

Add `"test": "vitest run"` to `packages/pipeline/package.json` scripts section.

**Step 3: Write the test file**

Create `packages/pipeline/src/write/credential-check.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { checkForCredentials } from "./credential-check.js";

describe("checkForCredentials", () => {
  it("returns found: false for clean text", () => {
    const result = checkForCredentials(
      "This is a normal code comment about APIs.",
    );
    expect(result.found).toBe(false);
    expect(result.patterns).toHaveLength(0);
  });

  it("detects API key patterns", () => {
    const result = checkForCredentials(
      'api_key = "sk_live_abc123def456ghi789jkl012mno"',
    );
    expect(result.found).toBe(true);
  });

  it("detects secret/token/password patterns", () => {
    const result = checkForCredentials('secret = "mySecretValue123"');
    expect(result.found).toBe(true);
  });

  it("detects Stripe-style keys (sk-/pk-)", () => {
    const result = checkForCredentials(
      "sk-proj_abcdefghijklmnopqrstuvwx",
    );
    expect(result.found).toBe(true);
  });

  it("detects GitHub tokens (ghp_)", () => {
    const result = checkForCredentials(
      "ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij",
    );
    expect(result.found).toBe(true);
  });

  it("detects Slack tokens (xoxb-)", () => {
    const result = checkForCredentials("xoxb-123456789-abcdefghijklm");
    expect(result.found).toBe(true);
  });

  it("detects JWT tokens", () => {
    const result = checkForCredentials(
      "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0",
    );
    expect(result.found).toBe(true);
  });

  it("detects AWS access key IDs", () => {
    const result = checkForCredentials("AKIAIOSFODNN7EXAMPLE");
    expect(result.found).toBe(true);
  });

  it("detects private key headers", () => {
    const result = checkForCredentials("-----BEGIN RSA PRIVATE KEY-----");
    expect(result.found).toBe(true);
    const result2 = checkForCredentials("-----BEGIN PRIVATE KEY-----");
    expect(result2.found).toBe(true);
  });

  it("detects database connection strings with credentials", () => {
    const result = checkForCredentials(
      "postgres://user:password@localhost:5432/db",
    );
    expect(result.found).toBe(true);
  });

  it("does not flag short values that are too small to be real keys", () => {
    const result = checkForCredentials('api_key = "test"');
    expect(result.found).toBe(false);
  });

  it("returns matched pattern sources in patterns array", () => {
    const result = checkForCredentials("AKIAIOSFODNN7EXAMPLE");
    expect(result.patterns.length).toBeGreaterThan(0);
    expect(result.patterns[0]).toContain("AKIA");
  });
});
```

**Step 4: Run to verify tests pass**

Run: `cd packages/pipeline && pnpm test`

Expected: PASS — 12 tests.

**Step 5: Commit**

```bash
git add packages/pipeline/vitest.config.ts packages/pipeline/package.json packages/pipeline/src/write/credential-check.test.ts
git commit -m "test(pipeline): add unit tests for credential detection"
```

---

### Task 10: Unit Tests for Cache (`packages/pipeline`)

**Files:**
- Create: `packages/pipeline/src/read/cache.test.ts`

**Context:** `packages/pipeline/src/read/cache.ts` exports `generateCacheKey(memoryId)`, `lookupCacheKey(key)`, and `clearExpiredKeys()`. Uses a module-level `Map<string, CacheEntry>` with 30-minute TTL. Keys are `ck_` prefixed. Uses `vi.useFakeTimers()` to control time — the cache module shares state across tests within a file, but fake timers let us control expiration precisely.

**Step 1: Write the test file**

Create `packages/pipeline/src/read/cache.test.ts`:

```ts
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  generateCacheKey,
  lookupCacheKey,
  clearExpiredKeys,
} from "./cache.js";

describe("cache", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("generateCacheKey returns a ck_ prefixed string", () => {
    const key = generateCacheKey("mem-1");
    expect(key).toMatch(/^ck_/);
  });

  it("generateCacheKey produces unique keys for the same memoryId", () => {
    const key1 = generateCacheKey("mem-1");
    const key2 = generateCacheKey("mem-1");
    expect(key1).not.toBe(key2);
  });

  it("lookupCacheKey returns memoryId for a valid key", () => {
    const key = generateCacheKey("mem-42");
    expect(lookupCacheKey(key)).toBe("mem-42");
  });

  it("lookupCacheKey returns null for unknown key", () => {
    expect(lookupCacheKey("ck_nonexistent")).toBeNull();
  });

  it("lookupCacheKey returns null for expired key", () => {
    const key = generateCacheKey("mem-expire");
    vi.advanceTimersByTime(31 * 60 * 1000);
    expect(lookupCacheKey(key)).toBeNull();
  });

  it("lookupCacheKey succeeds just before TTL expires", () => {
    const key = generateCacheKey("mem-boundary");
    vi.advanceTimersByTime(29 * 60 * 1000 + 59 * 1000);
    expect(lookupCacheKey(key)).toBe("mem-boundary");
  });

  it("clearExpiredKeys removes expired entries and keeps valid ones", () => {
    const oldKey = generateCacheKey("mem-old");
    vi.advanceTimersByTime(15 * 60 * 1000);
    const newKey = generateCacheKey("mem-new");
    vi.advanceTimersByTime(16 * 60 * 1000);

    clearExpiredKeys();

    expect(lookupCacheKey(oldKey)).toBeNull();
    expect(lookupCacheKey(newKey)).toBe("mem-new");
  });
});
```

**Step 2: Run to verify tests pass**

Run: `cd packages/pipeline && pnpm test`

Expected: PASS — credential-check + cache tests all green.

**Step 3: Commit**

```bash
git add packages/pipeline/src/read/cache.test.ts
git commit -m "test(pipeline): add unit tests for cache key lifecycle and TTL"
```

---

### Task 11: Unit Tests for Projections (`packages/pipeline`)

**Files:**
- Create: `packages/pipeline/src/projections.test.ts`

**Context:** `packages/pipeline/src/projections.ts` exports `projectSmall(memory, score)` and `projectMedium(memory, score)` — pure data projections that pick specific fields from a `Memory` object. `projectLarge` and `buildFeedbackSummary` call `@fixonce/storage` so we skip those (no API mocking per requirement).

**Step 1: Write the test file**

Create `packages/pipeline/src/projections.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import type { Memory } from "@fixonce/shared";
import { projectSmall, projectMedium } from "./projections.js";

function makeMemory(overrides?: Partial<Memory>): Memory {
  return {
    id: "550e8400-e29b-41d4-a716-446655440000",
    title: "Test Memory",
    content: "Full content here.",
    summary: "Short summary.",
    memory_type: "guidance",
    source_type: "discovery",
    created_by: "human",
    source_url: "https://example.com",
    tags: ["typescript"],
    language: "typescript",
    version_predicates: null,
    project_name: "fixonce",
    project_repo_url: "https://github.com/devrel-ai/fixonce",
    project_workspace_path: "/workspace",
    confidence: 0.8,
    surfaced_count: 5,
    last_surfaced_at: "2025-01-01T00:00:00Z",
    enabled: true,
    created_at: "2024-01-01T00:00:00Z",
    updated_at: "2024-06-01T00:00:00Z",
    embedding: null,
    ...overrides,
  };
}

describe("projectSmall", () => {
  it("returns only the small projection fields", () => {
    const memory = makeMemory();
    const result = projectSmall(memory, 0.95);

    expect(result).toEqual({
      id: memory.id,
      title: memory.title,
      content: memory.content,
      summary: memory.summary,
      memory_type: memory.memory_type,
      relevancy_score: 0.95,
    });
  });

  it("does not include medium/large fields", () => {
    const result = projectSmall(makeMemory(), 0.5);
    expect(result).not.toHaveProperty("tags");
    expect(result).not.toHaveProperty("language");
    expect(result).not.toHaveProperty("source_url");
    expect(result).not.toHaveProperty("confidence");
  });
});

describe("projectMedium", () => {
  it("includes small fields plus medium-specific fields", () => {
    const memory = makeMemory();
    const result = projectMedium(memory, 0.85);

    expect(result.id).toBe(memory.id);
    expect(result.relevancy_score).toBe(0.85);
    expect(result.tags).toEqual(["typescript"]);
    expect(result.language).toBe("typescript");
    expect(result.version_predicates).toBeNull();
    expect(result.created_by).toBe("human");
    expect(result.source_type).toBe("discovery");
    expect(result.created_at).toBe("2024-01-01T00:00:00Z");
    expect(result.updated_at).toBe("2024-06-01T00:00:00Z");
  });

  it("does not include large-only fields", () => {
    const result = projectMedium(makeMemory(), 0.5);
    expect(result).not.toHaveProperty("source_url");
    expect(result).not.toHaveProperty("confidence");
    expect(result).not.toHaveProperty("surfaced_count");
    expect(result).not.toHaveProperty("feedback_summary");
  });
});
```

**Step 2: Run to verify tests pass**

Run: `cd packages/pipeline && pnpm test`

Expected: PASS — all pipeline tests green.

**Step 3: Commit**

```bash
git add packages/pipeline/src/projections.test.ts
git commit -m "test(pipeline): add unit tests for verbosity projections"
```

---

### Task 12: Unit Tests for Environment Detection (`packages/pipeline`)

**Files:**
- Create: `packages/pipeline/src/environment.test.ts`

**Context:** `packages/pipeline/src/environment.ts` exports `detectEnvironment(input)`. It reads `package.json` and `compact.toml` via `node:fs/promises.readFile`, scans npm dependency names against `PACKAGE_MAP` (12 entries), strips version prefixes (`^`, `~`, `>=`), and returns detected/undetected components. We mock `node:fs/promises` — same pattern as existing `config.test.ts`.

**Step 1: Write the test file**

Create `packages/pipeline/src/environment.test.ts`:

```ts
import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("node:fs/promises", () => ({
  readFile: vi.fn(),
}));

import { readFile } from "node:fs/promises";
import { detectEnvironment } from "./environment.js";

const mockReadFile = vi.mocked(readFile);

describe("detectEnvironment", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockReadFile.mockRejectedValue(new Error("ENOENT"));
  });

  it("returns empty detected_versions when no files exist", async () => {
    const result = await detectEnvironment({ project_path: "/empty" });
    expect(Object.keys(result.detected_versions)).toHaveLength(0);
    expect(result.undetected_components).toHaveLength(12);
  });

  it("detects versions from package.json dependencies", async () => {
    mockReadFile.mockImplementation(async (filePath) => {
      if (String(filePath).endsWith("package.json")) {
        return JSON.stringify({
          dependencies: {
            "@aspect-build/compact-compiler": "^0.14.0",
            "@aspect-build/wallet-sdk": "~1.2.3",
          },
        });
      }
      throw new Error("ENOENT");
    });

    const result = await detectEnvironment({ project_path: "/project" });
    expect(result.detected_versions.compact_compiler).toBe("0.14.0");
    expect(result.detected_versions.wallet_sdk).toBe("1.2.3");
    expect(result.scan_sources.compact_compiler).toBe("package.json");
  });

  it("detects versions from devDependencies", async () => {
    mockReadFile.mockImplementation(async (filePath) => {
      if (String(filePath).endsWith("package.json")) {
        return JSON.stringify({
          devDependencies: {
            "@aspect-build/midnight-js": ">=2.0.0",
          },
        });
      }
      throw new Error("ENOENT");
    });

    const result = await detectEnvironment({ project_path: "/project" });
    expect(result.detected_versions.midnight_js).toBe("2.0.0");
  });

  it("detects compiler version from compact.toml", async () => {
    mockReadFile.mockImplementation(async (filePath) => {
      if (String(filePath).endsWith("compact.toml")) {
        return 'compiler_version = "0.14.0"';
      }
      throw new Error("ENOENT");
    });

    const result = await detectEnvironment({ project_path: "/project" });
    expect(result.detected_versions.compact_compiler).toBe("0.14.0");
    expect(result.scan_sources.compact_compiler).toBe("compact.toml");
  });

  it("package.json takes priority over compact.toml for same component", async () => {
    mockReadFile.mockImplementation(async (filePath) => {
      const path = String(filePath);
      if (path.endsWith("package.json")) {
        return JSON.stringify({
          dependencies: {
            "@aspect-build/compact-compiler": "^0.15.0",
          },
        });
      }
      if (path.endsWith("compact.toml")) {
        return 'compiler_version = "0.14.0"';
      }
      throw new Error("ENOENT");
    });

    const result = await detectEnvironment({ project_path: "/project" });
    expect(result.detected_versions.compact_compiler).toBe("0.15.0");
  });

  it("lists undetected components", async () => {
    mockReadFile.mockImplementation(async (filePath) => {
      if (String(filePath).endsWith("package.json")) {
        return JSON.stringify({
          dependencies: {
            "@aspect-build/compact-compiler": "^0.14.0",
          },
        });
      }
      throw new Error("ENOENT");
    });

    const result = await detectEnvironment({ project_path: "/project" });
    expect(result.undetected_components).toContain("wallet_sdk");
    expect(result.undetected_components).toContain("midnight_js");
    expect(result.undetected_components).not.toContain("compact_compiler");
    expect(result.undetected_components).toHaveLength(11);
  });
});
```

**Step 2: Run to verify tests pass**

Run: `cd packages/pipeline && pnpm test`

Expected: PASS — all pipeline tests green.

**Step 3: Commit**

```bash
git add packages/pipeline/src/environment.test.ts
git commit -m "test(pipeline): add unit tests for environment detection"
```

---

### Task 13: Unit & Integration Tests for Activity Stream (`packages/activity`)

**Files:**
- Create: `packages/activity/vitest.config.ts`
- Modify: `packages/activity/package.json:13-15` (add test script)
- Create: `packages/activity/src/stream.test.ts`

**Context:** `packages/activity/src/stream.ts` exports `subscribeToActivity(listener)` and `emitActivity(event)`. It's an in-memory pub/sub using a module-level `Set<ActivityListener>`. `subscribeToActivity` returns an unsubscribe function. `emitActivity` wraps each listener call in try/catch so one failing listener doesn't break others. Uses `vi.resetModules()` + dynamic `import()` to get fresh module state per test.

**Step 1: Create vitest config for activity package**

Create `packages/activity/vitest.config.ts`:

```ts
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    clearMocks: true,
  },
});
```

**Step 2: Add test script**

Add `"test": "vitest run"` to `packages/activity/package.json` scripts section.

**Step 3: Write the test file**

Create `packages/activity/src/stream.test.ts`:

```ts
import { describe, it, expect, vi } from "vitest";
import type { ActivityEvent } from "./stream.js";

async function loadStream() {
  vi.resetModules();
  return import("./stream.js");
}

function makeEvent(overrides?: Partial<ActivityEvent>): ActivityEvent {
  return {
    id: "evt-1",
    operation: "query",
    memory_id: null,
    details: {},
    created_at: "2024-01-01T00:00:00Z",
    ...overrides,
  };
}

describe("subscribeToActivity", () => {
  it("returns an unsubscribe function", async () => {
    const { subscribeToActivity } = await loadStream();
    const unsub = subscribeToActivity(() => {});
    expect(typeof unsub).toBe("function");
  });
});

describe("emitActivity", () => {
  it("calls subscribed listeners with the event", async () => {
    const { subscribeToActivity, emitActivity } = await loadStream();
    const listener = vi.fn();
    subscribeToActivity(listener);

    const event = makeEvent();
    emitActivity(event);

    expect(listener).toHaveBeenCalledOnce();
    expect(listener).toHaveBeenCalledWith(event);
  });

  it("calls multiple listeners", async () => {
    const { subscribeToActivity, emitActivity } = await loadStream();
    const listener1 = vi.fn();
    const listener2 = vi.fn();
    subscribeToActivity(listener1);
    subscribeToActivity(listener2);

    emitActivity(makeEvent());

    expect(listener1).toHaveBeenCalledOnce();
    expect(listener2).toHaveBeenCalledOnce();
  });

  it("does not call unsubscribed listeners", async () => {
    const { subscribeToActivity, emitActivity } = await loadStream();
    const listener = vi.fn();
    const unsub = subscribeToActivity(listener);

    unsub();
    emitActivity(makeEvent());

    expect(listener).not.toHaveBeenCalled();
  });

  it("continues calling other listeners when one throws", async () => {
    const { subscribeToActivity, emitActivity } = await loadStream();
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});

    const badListener = vi.fn(() => {
      throw new Error("listener failure");
    });
    const goodListener = vi.fn();

    subscribeToActivity(badListener);
    subscribeToActivity(goodListener);

    emitActivity(makeEvent());

    expect(badListener).toHaveBeenCalledOnce();
    expect(goodListener).toHaveBeenCalledOnce();
    expect(errorSpy).toHaveBeenCalled();

    errorSpy.mockRestore();
  });
});

describe("integration: multi-step lifecycle", () => {
  it("subscribe -> emit -> partial unsub -> emit -> verify", async () => {
    const { subscribeToActivity, emitActivity } = await loadStream();
    const listenerA = vi.fn();
    const listenerB = vi.fn();

    const unsubA = subscribeToActivity(listenerA);
    subscribeToActivity(listenerB);

    emitActivity(makeEvent({ id: "evt-1" }));
    expect(listenerA).toHaveBeenCalledTimes(1);
    expect(listenerB).toHaveBeenCalledTimes(1);

    unsubA();

    emitActivity(makeEvent({ id: "evt-2" }));
    expect(listenerA).toHaveBeenCalledTimes(1);
    expect(listenerB).toHaveBeenCalledTimes(2);
  });
});
```

**Step 4: Run to verify tests pass**

Run: `cd packages/activity && pnpm test`

Expected: PASS — 6 tests.

**Step 5: Commit**

```bash
git add packages/activity/vitest.config.ts packages/activity/package.json packages/activity/src/stream.test.ts
git commit -m "test(activity): add unit and integration tests for event stream"
```

---

### Task 14: GitHub Actions CI Workflow

**Files:**
- Create: `.github/workflows/ci.yml`

**Context:** Existing `.github/workflows/release.yml` uses Node 24 + pnpm v4 action + `actions/checkout@v4`. Mirror this setup. Turbo tasks `test` and `typecheck` have `"dependsOn": ["^build"]` so turbo auto-builds dependencies. The `lint` task has no dependencies.

**Step 1: Create CI workflow file**

Create `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  pull_request:
    branches: [main]

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

jobs:
  ci:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: pnpm/action-setup@v4

      - uses: actions/setup-node@v4
        with:
          node-version: "24"
          cache: pnpm

      - run: pnpm install --frozen-lockfile

      - name: Check formatting
        run: pnpm format:check

      - name: Lint
        run: pnpm lint

      - name: Type check
        run: pnpm typecheck

      - name: Test
        run: pnpm test
```

**Step 2: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add github actions workflow for PR checks"
```

---

### Task 15: Husky Pre-Push Hook

**Files:**
- Modify: `package.json` (add husky devDep + prepare script)
- Create: `.husky/pre-push`

**Step 1: Install husky**

Run:
```bash
pnpm add -Dw husky
```

**Step 2: Add prepare script to root package.json**

Add `"prepare": "husky"` to root `package.json` scripts section.

**Step 3: Initialize husky**

Run:
```bash
pnpm exec husky init
```

Expected: Creates `.husky/` directory with a default `pre-commit` hook.

**Step 4: Remove default pre-commit hook**

Run:
```bash
rm .husky/pre-commit
```

**Step 5: Create pre-push hook**

Create `.husky/pre-push`:

```sh
pnpm format:check
pnpm lint
pnpm typecheck
pnpm test
```

**Step 6: Commit**

```bash
git add -A
git commit -m "chore: add husky pre-push hook for local quality checks"
```

---

### Task 16: Final End-to-End Verification

**Step 1: Run all quality checks**

Run:
```bash
pnpm format:check
```

Expected: Exit 0, all files formatted correctly.

**Step 2: Run linter**

Run:
```bash
pnpm lint
```

Expected: Exit 0, zero errors, zero warnings.

**Step 3: Run type checker**

Run:
```bash
pnpm typecheck
```

Expected: Exit 0, no type errors across all packages.

**Step 4: Run all tests**

Run:
```bash
pnpm test
```

Expected: Exit 0, all tests pass across packages/shared, packages/storage, packages/pipeline, packages/activity.

**Step 5: Verify turbo caching works**

Run:
```bash
pnpm test
```

Expected: Turbo reports cache hits, completes near-instantly.

---

## Key Files Reference

| File | Purpose |
|------|---------|
| `eslint.config.js` | Root ESLint flat config (monorepo-wide) |
| `.prettierrc` | Prettier formatting config |
| `.prettierignore` | Prettier ignore patterns |
| `.github/workflows/ci.yml` | PR quality check workflow |
| `.husky/pre-push` | Local pre-push quality gate |
| `packages/shared/src/errors.test.ts` | FixOnceError unit tests |
| `packages/shared/src/schema.test.ts` | Zod schema validation tests |
| `packages/storage/src/version-filter.test.ts` | Version filter unit tests |
| `packages/pipeline/src/write/credential-check.test.ts` | Credential detection tests |
| `packages/pipeline/src/read/cache.test.ts` | Cache TTL unit tests |
| `packages/pipeline/src/projections.test.ts` | Verbosity projection tests |
| `packages/pipeline/src/environment.test.ts` | Environment detection tests |
| `packages/activity/src/stream.test.ts` | Event stream unit + integration tests |

## Existing Patterns to Reuse
- `packages/shared/vitest.config.ts` — vitest config pattern (`clearMocks: true`)
- `packages/shared/src/config.test.ts` — existing test style (`vi.mock` for `node:fs`)
- `.github/workflows/release.yml` — CI setup pattern (Node 24, pnpm v4 action)
