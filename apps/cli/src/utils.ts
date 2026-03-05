import { FixOnceError } from "@fixonce/shared";

export async function readStdin(): Promise<string> {
  const chunks: Buffer[] = [];
  for await (const chunk of process.stdin) {
    chunks.push(chunk as Buffer);
  }
  return Buffer.concat(chunks).toString("utf-8").trim();
}

export function handleError(err: unknown, jsonMode: boolean): never {
  if (err instanceof FixOnceError) {
    if (jsonMode) {
      console.error(JSON.stringify(err.toJSON(), null, 2));
    } else {
      console.error(`Error [${err.stage}]: ${err.message}`);
      console.error(`Suggestion: ${err.suggestion}`);
    }
  } else {
    const message = err instanceof Error ? err.message : String(err);
    if (jsonMode) {
      console.error(JSON.stringify({ stage: "unknown", reason: message, suggestion: "Check the error details and try again." }, null, 2));
    } else {
      console.error(`Error: ${message}`);
    }
  }

  return process.exit(1);
}
