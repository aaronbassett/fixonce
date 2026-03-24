/**
 * Zod schema validation wrapper.
 *
 * Parses `body` against the given schema and returns the typed result.
 * Throws a structured ValidationError on failure so callers can convert
 * it to a 400 error response.
 */
import { z } from "zod";

export class ValidationError extends Error {
  readonly issues: z.ZodIssue[];

  constructor(issues: z.ZodIssue[]) {
    const messages = issues.map((i) => `${i.path.join(".")}: ${i.message}`);
    super(messages.join("; "));
    this.name = "ValidationError";
    this.issues = issues;
  }
}

export function validateBody<S extends z.ZodTypeAny>(
  schema: S,
  body: unknown,
): z.output<S> {
  const result = schema.safeParse(body);
  if (!result.success) {
    throw new ValidationError(result.error.issues);
  }
  return result.data as z.output<S>;
}
