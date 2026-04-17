import { collectAppDiagnostics } from "./appDiagnostics";
import type { BenchmarkArtifactFile } from "./types";
import type { AppFilesState } from "./validators";

export function buildAppArtifacts(actual: AppFilesState): BenchmarkArtifactFile[] {
  const diagnostics = collectAppDiagnostics({
    frontend: actual.frontend,
    backend: actual.backend,
  });
  const artifacts: BenchmarkArtifactFile[] = [
    {
      path: "app.json",
      content: JSON.stringify(actual, null, 2) + "\n",
    },
    {
      path: "lint.json",
      content: JSON.stringify(diagnostics, null, 2) + "\n",
    },
  ];

  for (const [filePath, content] of Object.entries(actual.frontend)) {
    artifacts.push({
      path: `frontend${filePath.startsWith("/") ? filePath : `/${filePath}`}`,
      content,
    });
  }

  for (const [key, runnable] of Object.entries(actual.backend)) {
    artifacts.push({
      path: `backend/${key}/meta.json`,
      content: JSON.stringify(runnable, null, 2) + "\n",
    });

    const inlineContent = runnable.inlineScript?.content;
    if (inlineContent) {
      const extension = runnable.inlineScript?.language === "python3" ? "py" : "ts";
      artifacts.push({
        path: `backend/${key}/main.${extension}`,
        content: inlineContent,
      });
    }
  }

  if (actual.datatables.length > 0) {
    artifacts.push({
      path: "datatables.json",
      content: JSON.stringify(actual.datatables, null, 2) + "\n",
    });
  }

  return artifacts;
}
