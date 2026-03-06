import OpenAI from "openai";
import { getConfig } from "@fixonce/shared";

export interface LLMConfig {
  model: string;
  temperature?: number;
  maxTokens?: number;
  timeoutMs?: number;
}

const DEFAULT_CONFIGS: Record<string, LLMConfig> = {
  quality_gate: {
    model: "google/gemma-3-4b-it",
    temperature: 0.1,
    maxTokens: 500,
    timeoutMs: 10000,
  },
  duplicate_detection: {
    model: "anthropic/claude-3.5-haiku",
    temperature: 0.1,
    maxTokens: 1000,
    timeoutMs: 10000,
  },
  query_rewriting: {
    model: "google/gemma-3-4b-it",
    temperature: 0.3,
    maxTokens: 300,
    timeoutMs: 10000,
  },
  reranking: {
    model: "google/gemma-3-4b-it",
    temperature: 0.1,
    maxTokens: 2000,
    timeoutMs: 10000,
  },
};

let openrouterClient: OpenAI | null = null;

function getClient(): OpenAI {
  if (openrouterClient) return openrouterClient;

  const { openrouterApiKey } = getConfig();
  openrouterClient = new OpenAI({
    baseURL: "https://openrouter.ai/api/v1",
    apiKey: openrouterApiKey,
    defaultHeaders: {
      "X-Title": "fixonce",
    },
  });
  return openrouterClient;
}

export async function llmCall(
  taskType: string,
  systemPrompt: string,
  userMessage: string,
  configOverride?: Partial<LLMConfig>,
): Promise<string> {
  const client = getClient();
  const config = { ...DEFAULT_CONFIGS[taskType], ...configOverride };

  if (!config.model) {
    throw new Error(`No default model configured for task type: ${taskType}`);
  }

  const response = await client.chat.completions.create({
    model: config.model,
    messages: [
      { role: "system", content: systemPrompt },
      { role: "user", content: userMessage },
    ],
    temperature: config.temperature ?? 0.1,
    max_tokens: config.maxTokens ?? 500,
  });

  const content = response.choices[0]?.message?.content;
  if (!content) {
    throw new Error(`LLM returned empty response for task: ${taskType}`);
  }

  return content;
}

export async function llmCallJSON<T>(
  taskType: string,
  systemPrompt: string,
  userMessage: string,
  configOverride?: Partial<LLMConfig>,
): Promise<T> {
  const raw = await llmCall(
    taskType,
    systemPrompt,
    userMessage,
    configOverride,
  );

  // Extract JSON from response (may be wrapped in markdown code blocks)
  const jsonMatch = raw.match(/```(?:json)?\s*\n?([\s\S]*?)\n?```/) || [
    null,
    raw,
  ];
  // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition -- fallback array makes [1] always defined, but regex match type is wider
  const jsonStr = jsonMatch[1]?.trim() ?? raw.trim();

  try {
    return JSON.parse(jsonStr) as T;
  } catch {
    throw new Error(
      `Failed to parse LLM JSON response for ${taskType}: ${jsonStr.slice(0, 200)}`,
    );
  }
}

export function resetLLMClient(): void {
  openrouterClient = null;
}
