import { VoyageAIClient } from "voyageai";

let voyageClient: VoyageAIClient | null = null;

function getVoyageClient(): VoyageAIClient {
  if (voyageClient) return voyageClient;

  const apiKey = process.env.VOYAGE_API_KEY;
  if (!apiKey) {
    throw new Error(
      "VOYAGE_API_KEY is not set. Add it to your .env file. " +
        "Get it from https://dash.voyageai.com/",
    );
  }

  voyageClient = new VoyageAIClient({ apiKey });
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

  return result.data![0]!.embedding!;
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

  return result.data!.map((d) => d.embedding!);
}

export function resetVoyageClient(): void {
  voyageClient = null;
}
