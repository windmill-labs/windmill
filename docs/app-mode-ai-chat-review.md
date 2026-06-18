# App Mode AI Chat Review

This note only tracks the highest-value next steps for making app-mode AI chat
safer and more efficient.

## Recommended Next Steps

1. Add confirmation for dangerous app tools.

   Require explicit user confirmation before file writes, file deletes, backend
   runnable writes, backend runnable deletes, and datatable SQL execution. Show a
   useful diff or exact SQL before applying the action.

2. Enforce datatable SQL safety in code.

   Do not rely on prompt instructions for SQL safety. Classify statements before
   execution, block DDL unless table creation is allowed, and require
   confirmation for DDL, DML, and row-returning reads that would expose data back
   to the model.

3. Keep default context demand-driven.

   Prefer selected context and targeted reads before broad discovery. Keep file
   listings metadata-only, avoid sending full datatable schemas by default, and
   keep SDK/reference material out of the base prompt unless it is requested or
   needed for the task.

4. Improve app context lifecycle.

   Treat `@` context as per-message by default, with an explicit pinning affordance
   for context that should persist. Lazy-load file and runnable contents, and add
   a visible approximate context-size indicator so users can spot prompt bloat.

5. Refresh datatable context after mutations.

   Refresh table metadata after data-panel changes and after AI-created tables so
   follow-up tool calls and user-visible context do not use stale schema data.

6. Persist table creation policy explicitly.

   Store whether AI table creation is enabled as an explicit app setting instead
   of inferring it from the presence of datatable configuration.

7. Add focused eval coverage for these behaviors.

   Cover confirmation requirements, datatable SQL policy enforcement, selected
   context minimization, and stale-schema refresh behavior with targeted app-mode
   evals or lower-level tests where practical.
