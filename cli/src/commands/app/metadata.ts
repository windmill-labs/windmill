export type Runnable =
  | {
      name: string;
      type?: "runnableByName" | "inline";
      path?: string;
      inlineScript?: {
        content: string;
        language: string;
        lock?: string;
        cache_ttl?: number;
        id?: number;
      };
      fields?: Record<string, any>;
    }
  | {
      type: "path";
      runType: "script" | "hubscript" | "flow";
      path: string;
      fields?: Record<string, any>;
    };
