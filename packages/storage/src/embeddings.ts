import { VoyageAIClient } from "voyageai";
import { getConfig } from "@fixonce/shared";

let voyageClient: VoyageAIClient | null = null;

function getVoyageClient(): VoyageAIClient {
  if (voyageClient) return voyageClient;

  const { voyageApiKey } = getConfig();
  voyageClient = new VoyageAIClient({ apiKey: voyageApiKey });
  return voyageClient;
}

export async function generateEmbedding(
  text: string,
  inputType: "document" | "query" = "document",
): Promise<number[]> {
  const client = getVoyageClient();
  const result = await client.embed({
    input: [text],
    model: "voyage-code-3",
    inputType,
    outputDimension: 1024,
  });

  const embedding = result.data?.[0]?.embedding;
  if (!embedding) {
    throw new Error("VoyageAI returned no embedding for input text");
  }
  return embedding;
}

export async function generateEmbeddings(
  texts: string[],
  inputType: "document" | "query" = "document",
): Promise<number[][]> {
  if (texts.length === 0) return [];

  const client = getVoyageClient();
  const result = await client.embed({
    input: texts,
    model: "voyage-code-3",
    inputType,
    outputDimension: 1024,
  });

  if (!result.data) {
    throw new Error("VoyageAI returned no data for batch embedding request");
  }
  return result.data.map((d, i) => {
    if (!d.embedding) {
      throw new Error(`VoyageAI returned no embedding for input at index ${i}`);
    }
    return d.embedding;
  });
}

export function resetVoyageClient(): void {
  voyageClient = null;
}
