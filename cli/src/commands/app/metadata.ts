export type Runnable =
  | {
      name: string;
      type?: "runnableByName";
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
      type: "runnableByPath";
      path: string;
      runType?: "script" | "flow" | "hubscript";
      fields?: Record<string, any>;
      schema?: any;
    };
