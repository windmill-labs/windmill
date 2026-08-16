import express from "express";
import { serve } from "inngest/express";
import { inngest, benchmarkFn } from "./workflow";

const app = express();

// Inngest needs the raw body for signature verification
app.use(express.json());

app.use(
  "/api/inngest",
  serve({ client: inngest, functions: [benchmarkFn] }),
);

app.get("/health", (_req, res) => res.send("ok"));

app.listen(3000, () => {
  console.log("Inngest app server listening on port 3000");
});
