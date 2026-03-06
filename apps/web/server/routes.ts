import { Router } from "express";
import type { Request, Response } from "express";
import {
  createMemory,
  queryMemories,
  expandCacheKey,
  getMemory,
  updateMemory,
  submitFeedback,
  detectEnvironment,
  detectDuplicates,
} from "@fixonce/pipeline";
import {
  deleteMemory,
  listFeedbackByMemoryId,
  listActivity,
  updateMemory as storageUpdateMemory,
} from "@fixonce/storage";
import { FixOnceError } from "@fixonce/shared";
import type { OperationType } from "@fixonce/shared";

const router = Router();

function str(value: unknown): string | undefined {
  if (typeof value === "string") return value;
  if (Array.isArray(value) && typeof value[0] === "string") return value[0];
  return undefined;
}

function handleError(res: Response, err: unknown): void {
  if (err instanceof FixOnceError) {
    res.status(400).json({ error: err.toJSON() });
    return;
  }
  console.error("Unexpected error:", err);
  res.status(500).json({
    error: {
      stage: "unknown",
      reason: err instanceof Error ? err.message : "Internal server error",
      suggestion: "Check the server logs for more details.",
    },
  });
}

// GET /api/memories — queryMemories (query params)
router.get("/memories", async (req: Request, res: Response) => {
  try {
    const q = req.query;
    const tagsRaw = str(q.tags);

    const result = await queryMemories({
      query: str(q.query) ?? "",
      rewrite:
        str(q.rewrite) === "true"
          ? true
          : str(q.rewrite) === "false"
            ? false
            : undefined,
      type: str(q.type) as "simple" | "vector" | "hybrid" | undefined,
      rerank:
        str(q.rerank) === "true"
          ? true
          : str(q.rerank) === "false"
            ? false
            : undefined,
      tags: tagsRaw ? tagsRaw.split(",") : undefined,
      language: str(q.language),
      project_name: str(q.project_name),
      memory_type: str(q.memory_type) as
        | "guidance"
        | "anti_pattern"
        | undefined,
      created_after: str(q.created_after),
      updated_after: str(q.updated_after),
      max_results: str(q.max_results) ? Number(str(q.max_results)) : undefined,
      max_tokens: str(q.max_tokens) ? Number(str(q.max_tokens)) : undefined,
      verbosity: str(q.verbosity) as "small" | "medium" | "large" | undefined,
    });
    res.json(result);
  } catch (err) {
    handleError(res, err);
  }
});

// POST /api/memories — createMemory (force created_by: "human")
router.post("/memories", async (req: Request, res: Response) => {
  try {
    const result = await createMemory({
      ...req.body,
      created_by: "human",
    });
    res.status(201).json(result);
  } catch (err) {
    handleError(res, err);
  }
});

// GET /api/memories/:id — getMemory
router.get("/memories/:id", async (req: Request, res: Response) => {
  try {
    const verbosity = str(req.query.verbosity) as
      | "small"
      | "medium"
      | "large"
      | undefined;
    const id = String(req.params.id);
    const result = await getMemory({
      id,
      verbosity,
    });
    res.json(result);
  } catch (err) {
    handleError(res, err);
  }
});

// PATCH /api/memories/:id — updateMemory (sets created_by: "human_modified")
router.patch("/memories/:id", async (req: Request, res: Response) => {
  try {
    const id = String(req.params.id);
    const result = await updateMemory({
      id,
      ...req.body,
    });

    // Mark as human_modified since this came through the web UI
    await storageUpdateMemory(id, { created_by: "human_modified" });

    res.json(result);
  } catch (err) {
    handleError(res, err);
  }
});

// DELETE /api/memories/:id — deleteMemory (hard delete, web-only)
router.delete("/memories/:id", async (req: Request, res: Response) => {
  try {
    await deleteMemory(String(req.params.id));
    res.status(204).end();
  } catch (err) {
    handleError(res, err);
  }
});

// POST /api/memories/:id/feedback — submitFeedback
router.post("/memories/:id/feedback", async (req: Request, res: Response) => {
  try {
    const result = await submitFeedback({
      memory_id: String(req.params.id),
      ...req.body,
    });
    res.status(201).json(result);
  } catch (err) {
    handleError(res, err);
  }
});

// GET /api/memories/:id/feedback — listFeedbackByMemoryId
router.get("/memories/:id/feedback", async (req: Request, res: Response) => {
  try {
    const feedback = await listFeedbackByMemoryId(String(req.params.id));
    res.json(feedback);
  } catch (err) {
    handleError(res, err);
  }
});

// GET /api/activity — listActivity with pagination
router.get("/activity", async (req: Request, res: Response) => {
  try {
    const operation = str(req.query.operation) as OperationType | undefined;
    const memoryId = str(req.query.memory_id);
    const limitStr = str(req.query.limit);
    const offsetStr = str(req.query.offset);
    const since = str(req.query.since);

    const logs = await listActivity({
      operation,
      memory_id: memoryId,
      limit: limitStr ? Number(limitStr) : 50,
      offset: offsetStr ? Number(offsetStr) : undefined,
      since,
    });
    res.json(logs);
  } catch (err) {
    handleError(res, err);
  }
});

// GET /api/environment — detectEnvironment
router.get("/environment", async (req: Request, res: Response) => {
  try {
    const projectPath = str(req.query.project_path);
    const result = await detectEnvironment({
      project_path: projectPath,
    });
    res.json(result);
  } catch (err) {
    handleError(res, err);
  }
});

// GET /api/expand/:cache_key — expandCacheKey
router.get("/expand/:cache_key", async (req: Request, res: Response) => {
  try {
    const verbosity = str(req.query.verbosity) as
      | "small"
      | "medium"
      | "large"
      | undefined;
    const result = await expandCacheKey({
      cache_key: String(req.params.cache_key),
      verbosity,
    });
    res.json(result);
  } catch (err) {
    handleError(res, err);
  }
});

// POST /api/memories/preview-duplicates — preview duplicate candidates
router.post(
  "/memories/preview-duplicates",
  async (req: Request, res: Response) => {
    try {
      const { title, content, summary, language } = req.body;
      const result = await detectDuplicates(
        String(title ?? ""),
        String(content ?? ""),
        String(summary ?? ""),
        String(language ?? "typescript"),
      );
      res.json(result);
    } catch (err) {
      handleError(res, err);
    }
  },
);

export { router };
