import { describe, it, expect } from "vitest";
import { checkForCredentials } from "./credential-check.js";

describe("checkForCredentials", () => {
  it("returns found: false for clean text", () => {
    const result = checkForCredentials("This is perfectly normal text with no secrets.");
    expect(result.found).toBe(false);
    expect(result.patterns).toEqual([]);
  });

  it("detects API key patterns", () => {
    const result = checkForCredentials('api_key = "abcdefghijklmnopqrstuvwxyz"');
    expect(result.found).toBe(true);
    expect(result.patterns.length).toBeGreaterThan(0);
  });

  it("detects secret/token/password patterns", () => {
    const result = checkForCredentials('secret = "mySecretValue123"');
    expect(result.found).toBe(true);

    const result2 = checkForCredentials('token = "abcdef1234567890"');
    expect(result2.found).toBe(true);

    const result3 = checkForCredentials('password = "hunter2abc"');
    expect(result3.found).toBe(true);
  });

  it("detects Stripe-style keys (sk-/pk-)", () => {
    const result = checkForCredentials("sk-1234567890abcdefghijklmnop");
    expect(result.found).toBe(true);

    const result2 = checkForCredentials("pk-1234567890abcdefghijklmnop");
    expect(result2.found).toBe(true);
  });

  it("detects GitHub tokens (ghp_)", () => {
    const result = checkForCredentials("ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmn");
    expect(result.found).toBe(true);
  });

  it("detects Slack tokens (xoxb-)", () => {
    const result = checkForCredentials("xoxb-123456789012-1234567890123-abcdefghijklmnop");
    expect(result.found).toBe(true);
  });

  it("detects JWT tokens", () => {
    const result = checkForCredentials(
      "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0"
    );
    expect(result.found).toBe(true);
  });

  it("detects AWS access key IDs (AKIA...)", () => {
    const result = checkForCredentials("AKIAIOSFODNN7EXAMPLE");
    expect(result.found).toBe(true);
  });

  it("detects private key headers", () => {
    const result = checkForCredentials("-----BEGIN RSA PRIVATE KEY-----");
    expect(result.found).toBe(true);

    const result2 = checkForCredentials("-----BEGIN PRIVATE KEY-----");
    expect(result2.found).toBe(true);

    const result3 = checkForCredentials("-----BEGIN EC PRIVATE KEY-----");
    expect(result3.found).toBe(true);
  });

  it("detects database connection strings with credentials", () => {
    const result = checkForCredentials("mongodb://admin:password123@localhost:27017/mydb");
    expect(result.found).toBe(true);

    const result2 = checkForCredentials("postgres://user:secret@db.example.com:5432/prod");
    expect(result2.found).toBe(true);
  });

  it("does not flag short values that are too small to be real keys", () => {
    const result = checkForCredentials('api_key = "short"');
    expect(result.found).toBe(false);

    const result2 = checkForCredentials('secret = "abc"');
    expect(result2.found).toBe(false);
  });

  it("returns matched pattern sources in patterns array", () => {
    const result = checkForCredentials("AKIAIOSFODNN7EXAMPLE");
    expect(result.patterns).toContain("AKIA[0-9A-Z]{16}");

    const result2 = checkForCredentials("-----BEGIN RSA PRIVATE KEY-----");
    expect(result2.patterns.length).toBe(1);
    expect(result2.patterns[0]).toContain("PRIVATE KEY");
  });
});
