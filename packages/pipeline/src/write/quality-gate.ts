import { llmCallJSON } from "../llm.js";
import { checkForCredentials } from "./credential-check.js";

interface QualityGateResult {
  decision: "accept" | "reject";
  reason: string;
}

const SYSTEM_PROMPT = `You are a quality evaluator for a memory store used by LLM coding agents.

Evaluate whether the submitted memory is worth storing. Return JSON:
{"decision": "accept" | "reject", "reason": "brief explanation"}

REJECT when:
- Too vague (e.g., "be careful with types")
- Too specific to a single line of code with no generalizable lesson
- Trivially obvious to any developer
- Contains only code without explanation of why it matters

ACCEPT when:
- Actionable: tells the agent what to do or avoid
- Generalizable: applies beyond the specific instance
- Contains a "why" not just a "what"
- Captures a non-obvious lesson or gotcha`;

export async function evaluateQuality(
  title: string,
  content: string,
  summary: string,
): Promise<QualityGateResult> {
  // Check for credentials first
  const credCheck = checkForCredentials(content);
  if (credCheck.found) {
    return {
      decision: "reject",
      reason: "Memory content contains potential credentials or secrets",
    };
  }

  const titleCredCheck = checkForCredentials(title);
  if (titleCredCheck.found) {
    return {
      decision: "reject",
      reason: "Memory title contains potential credentials or secrets",
    };
  }

  const summaryCredCheck = checkForCredentials(summary);
  if (summaryCredCheck.found) {
    return {
      decision: "reject",
      reason: "Memory summary contains potential credentials or secrets",
    };
  }

  const userMessage = `Title: ${title}\nSummary: ${summary}\n\nContent:\n${content}`;

  return llmCallJSON<QualityGateResult>(
    "quality_gate",
    SYSTEM_PROMPT,
    userMessage,
  );
}
