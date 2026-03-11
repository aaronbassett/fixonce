import express from "express";
import { router } from "./routes.js";
import { subscribeToActivityRealtime } from "@fixonce/activity";
import type { ActivityEvent } from "@fixonce/activity";

const app = express();
app.use(express.json());
app.use("/api", router);

// SSE endpoint for realtime activity stream
app.get("/api/activity/stream", (req, res) => {
  res.setHeader("Content-Type", "text/event-stream");
  res.setHeader("Cache-Control", "no-cache");
  res.setHeader("Connection", "keep-alive");

  const unsubscribe = subscribeToActivityRealtime((event: ActivityEvent) => {
    res.write(`data: ${JSON.stringify(event)}\n\n`);
  });

  req.on("close", unsubscribe);
});

const port = parseInt(process.env.PORT ?? "3001", 10);
app.listen(port, () => {
  console.log(`FixOnce Web UI API running at http://localhost:${port}`);
});
