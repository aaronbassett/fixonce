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
    const result = checkForCredentials("sk-abcdefghijklmnopqrstuvwx");
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
