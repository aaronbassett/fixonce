import type {
  CreateMemoryInput,
  CreateMemoryResult,
  QueryMemoriesResult,
  GetMemoryResult,
  UpdateMemoryResult,
  SubmitFeedbackInput,
  SubmitFeedbackResult,
  DetectEnvironmentResult,
  ExpandCacheKeyResult,
  Feedback,
  ActivityLog,
} from "@fixonce/shared";

const BASE = "/api";

async function request<T>(url: string, options?: RequestInit): Promise<T> {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...(options?.headers as Record<string, string> | undefined),
  };
  const res = await fetch(url, {
    ...options,
    headers,
  });

  if (!res.ok) {
    const body = await res.json().catch(() => null);
    const message =
      body?.error?.reason ?? `Request failed with status ${res.status}`;
    throw new Error(message);
  }

  if (res.status === 204) {
    return undefined as T;
  }

  return res.json() as Promise<T>;
}

export async function fetchMemories(
  params: Record<string, string>,
): Promise<QueryMemoriesResult> {
  const search = new URLSearchParams(params);
  return request<QueryMemoriesResult>(`${BASE}/memories?${search.toString()}`);
}

export async function createMemoryApi(
  input: Omit<CreateMemoryInput, "created_by">,
): Promise<CreateMemoryResult> {
  return request<CreateMemoryResult>(`${BASE}/memories`, {
    method: "POST",
    body: JSON.stringify(input),
  });
}

export async function getMemoryApi(
  id: string,
  verbosity?: string,
): Promise<GetMemoryResult> {
  const params = verbosity ? `?verbosity=${verbosity}` : "";
  return request<GetMemoryResult>(`${BASE}/memories/${id}${params}`);
}

export async function updateMemoryApi(
  id: string,
  updates: Record<string, unknown>,
): Promise<UpdateMemoryResult> {
  return request<UpdateMemoryResult>(`${BASE}/memories/${id}`, {
    method: "PATCH",
    body: JSON.stringify(updates),
  });
}

export async function deleteMemoryApi(id: string): Promise<void> {
  return request<undefined>(`${BASE}/memories/${id}`, {
    method: "DELETE",
  });
}

export async function submitFeedbackApi(
  input: SubmitFeedbackInput,
): Promise<SubmitFeedbackResult> {
  return request<SubmitFeedbackResult>(
    `${BASE}/memories/${input.memory_id}/feedback`,
    {
      method: "POST",
      body: JSON.stringify({
        text: input.text,
        tags: input.tags,
        suggested_action: input.suggested_action,
      }),
    },
  );
}

export async function getFeedbackApi(id: string): Promise<Feedback[]> {
  return request<Feedback[]>(`${BASE}/memories/${id}/feedback`);
}

export async function getActivityApi(
  params?: Record<string, string>,
): Promise<ActivityLog[]> {
  const search = params ? `?${new URLSearchParams(params).toString()}` : "";
  return request<ActivityLog[]>(`${BASE}/activity${search}`);
}

export async function detectEnvironmentApi(): Promise<DetectEnvironmentResult> {
  return request<DetectEnvironmentResult>(`${BASE}/environment`);
}

export async function expandCacheKeyApi(
  key: string,
  verbosity?: string,
): Promise<ExpandCacheKeyResult> {
  const params = verbosity ? `?verbosity=${verbosity}` : "";
  return request<ExpandCacheKeyResult>(`${BASE}/expand/${key}${params}`);
}

export async function previewDuplicatesApi(input: {
  title: string;
  content: string;
  summary: string;
  language: string;
}): Promise<{
  outcome: string;
  reason: string;
  target_memory_id?: string;
}> {
  return request(`${BASE}/memories/preview-duplicates`, {
    method: "POST",
    body: JSON.stringify(input),
  });
}
