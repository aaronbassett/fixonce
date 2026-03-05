/**
 * Structured error type returned by all service layer operations.
 * Includes stage identification and actionable suggestions per
 * Constitution Principle V (Fail Fast with Actionable Errors).
 */
export interface ServiceError {
  stage: string;
  reason: string;
  suggestion: string;
}

export class FixOnceError extends Error {
  public readonly stage: string;
  public readonly suggestion: string;

  constructor(error: ServiceError) {
    super(error.reason);
    this.name = "FixOnceError";
    this.stage = error.stage;
    this.suggestion = error.suggestion;
  }

  toJSON(): ServiceError {
    return {
      stage: this.stage,
      reason: this.message,
      suggestion: this.suggestion,
    };
  }
}

export function validationError(reason: string, suggestion: string): FixOnceError {
  return new FixOnceError({ stage: "validation", reason, suggestion });
}

export function storageError(reason: string, suggestion: string): FixOnceError {
  return new FixOnceError({ stage: "storage", reason, suggestion });
}

export function qualityGateError(reason: string, suggestion: string): FixOnceError {
  return new FixOnceError({ stage: "quality_gate", reason, suggestion });
}

export function duplicateDetectionError(reason: string, suggestion: string): FixOnceError {
  return new FixOnceError({ stage: "duplicate_detection", reason, suggestion });
}

export function searchError(reason: string, suggestion: string): FixOnceError {
  return new FixOnceError({ stage: "search", reason, suggestion });
}

export function rewriteError(reason: string, suggestion: string): FixOnceError {
  return new FixOnceError({ stage: "rewrite", reason, suggestion });
}

export function rerankError(reason: string, suggestion: string): FixOnceError {
  return new FixOnceError({ stage: "rerank", reason, suggestion });
}

export function embeddingError(reason: string, suggestion: string): FixOnceError {
  return new FixOnceError({ stage: "embedding", reason, suggestion });
}
