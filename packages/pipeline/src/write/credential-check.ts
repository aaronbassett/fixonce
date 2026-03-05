const CREDENTIAL_PATTERNS = [
  /(?:api[_-]?key|apikey)\s*[:=]\s*['"]?[a-zA-Z0-9_\-]{20,}/i,
  /(?:secret|token|password|passwd|pwd)\s*[:=]\s*['"]?[a-zA-Z0-9_\-]{8,}/i,
  /(?:sk|pk|rk)[-_][a-zA-Z0-9]{20,}/,
  /(?:ghp|gho|ghu|ghs|ghr)_[a-zA-Z0-9]{36,}/,
  /(?:xox[bpors])-[a-zA-Z0-9-]+/,
  /eyJ[a-zA-Z0-9_-]{10,}\.[a-zA-Z0-9_-]{10,}/,
  /AKIA[0-9A-Z]{16}/,
  /-----BEGIN (?:RSA |EC |DSA )?PRIVATE KEY-----/,
  /(?:mongodb|postgres|mysql|redis):\/\/[^\s]+:[^\s]+@/i,
];

export function checkForCredentials(text: string): { found: boolean; patterns: string[] } {
  const found: string[] = [];

  for (const pattern of CREDENTIAL_PATTERNS) {
    if (pattern.test(text)) {
      found.push(pattern.source.slice(0, 40));
    }
  }

  return { found: found.length > 0, patterns: found };
}
