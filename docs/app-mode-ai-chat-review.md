# App Mode AI Chat Review

## Purpose

This document reviews the current app-mode AI chat design with a focus on:

- keeping prompts and context as small as possible;
- requiring user confirmation for important actions;
- making datatable integration smooth and safe for users.

## Short verdict

The app-mode AI chat has a solid foundation: mode-specific helpers, explicit `@` context, app snapshots/revert, datatable whitelisting, and generic confirmation UI already exist.

However, it is not yet optimal for minimal context and user-safe automation:

1. **Context is still too large by default**, especially the app system prompt, broad file-discovery guidance, full datatable schemas, and persistent `@` context. (`get_files()` has since been replaced by metadata-only `list_files()`.)
2. **Important app/datatable actions are not consistently confirmed**. The confirmation infrastructure exists, but app tools mostly bypass it.
3. **Datatables UX is promising but has rough edges**: stale cached table context, weak SQL safety, policy persistence issues, and too-heavy full-schema fetching.

## Relevant files

### AI chat orchestration

- `frontend/src/lib/components/copilot/chat/AIChatManager.svelte.ts`
- `frontend/src/lib/components/copilot/chat/chatLoop.ts`
- `frontend/src/lib/components/copilot/chat/shared.ts`
- `frontend/src/lib/components/copilot/chat/AIChat.svelte`
- `frontend/src/lib/components/copilot/chat/AIChatDisplay.svelte`
- `frontend/src/lib/components/copilot/chat/AIChatInput.svelte`
- `frontend/src/lib/components/copilot/chat/ToolExecutionDisplay.svelte`

### App mode

- `frontend/src/lib/components/copilot/chat/app/core.ts`
- `frontend/src/lib/components/copilot/chat/AppAvailableContextList.svelte`
- `frontend/src/lib/components/copilot/chat/ContextElementBadge.svelte`
- `frontend/src/lib/components/copilot/chat/DatatableCreationPolicy.svelte`

### Raw app editor and datatables

- `frontend/src/lib/components/raw_apps/RawAppEditor.svelte`
- `frontend/src/lib/components/raw_apps/RawAppDataTableList.svelte`
- `frontend/src/lib/components/raw_apps/RawAppDataTableDrawer.svelte`
- `frontend/src/lib/components/raw_apps/DefaultDatabaseSelector.svelte`
- `frontend/src/lib/components/raw_apps/dataTableRefUtils.ts`
- `frontend/src/lib/components/raw_apps/datatableUtils.svelte.ts`
- `frontend/src/routes/(root)/(logged)/apps_raw/add/+page.svelte`
- `frontend/src/routes/(root)/(logged)/apps_raw/edit/[...path]/+page.svelte`

### Backend datatable APIs

- `backend/windmill-api-workspaces/src/workspaces.rs`
  - `list_datatables`
  - `list_datatable_schemas`
  - `get_datatable_schema`
  - `edit_datatable_config`

### System prompts

- `system_prompts/README.md`
- `system_prompts/auto-generated/index.ts`
- `system_prompts/auto-generated/sdks/datatable-typescript.md`
- `system_prompts/auto-generated/sdks/datatable-python.md`

## How app mode works today

In the raw app editor, `RawAppEditor.svelte` initializes app-mode AI chat on mount:

- calls `aiChatManager.saveAndClear()`;
- calls `aiChatManager.changeMode(AIMode.APP)`;
- registers app helpers through `aiChatManager.setAppHelpers(...)`.

Those app helpers expose operations for:

- frontend files;
- backend runnables;
- current selected editor context;
- linting;
- app snapshots and revert;
- datatable schema loading;
- SQL execution;
- app table whitelisting.

When app mode is active, `AIChatManager.changeMode(AIMode.APP)` sets:

- system prompt: `prepareAppSystemMessage(...)`;
- tools: `getAppTools()`;
- helpers: `appAiChatHelpers`.

When the user sends a message, `prepareAppUserMessage(...)` builds the user prompt from:

- current frontend/backend file selection, unless excluded;
- inspector-selected DOM element;
- editor code selection;
- additional `@`-mentioned context;
- the user instructions.

`runChatLoop(...)` then sends the system message, history, user message, and tool definitions to the selected model. Tool calls go through `processToolCall(...)`, which supports confirmation only when a tool opts into `requiresConfirmation`.

## Current app tools

### Read and discovery tools

These are generally safe without confirmation:

- `list_files`
- `get_frontend_file`
- `get_backend_runnable`
- `get_selected_context`
- `lint`
- `search_workspace`
- `get_runnable_details`
- `search_hub_scripts`
- `list_datatables`
- `get_datatable_table_schema`

### Mutating tools

These currently execute directly in app mode:

- `set_frontend_file`
- `patch_file`
- `delete_frontend_file`
- `set_backend_runnable`
- `delete_backend_runnable`
- `exec_datatable_sql`

This is the biggest mismatch with the requirement that every important action should be confirmed by the user.

## System prompt assessment

The app system prompt is useful but heavier than ideal.

### Strengths

- Clearly explains raw app structure.
- Explains the frontend/backend runnable split.
- Encourages `patch_file` for small edits.
- Pushes datatables for persisted app storage.
- Explains that datatable DDL should go through `exec_datatable_sql`.
- Includes table creation policy context.

### Concerns

1. It always includes broad app-building instructions, even for small localized edits.
2. The previous prompt included the datatable SDK reference for both TypeScript and Python every time. This has since been removed; concise examples remain in the prompt.
3. The previous prompt told the model to start with `get_files()`, which encouraged loading all files even when selected context was sufficient. This is now improved by `list_files()`, but the prompt still needs to stay demand-driven.
4. It relies heavily on prompt instructions for datatable safety instead of enforcing safety in tools.
5. Custom workspace/user prompts are appended as `USER GIVEN INSTRUCTIONS`, which is flexible but can further increase context.

### Recommendation

The base app prompt should be shorter and more demand-driven:

- Keep file discovery demand-driven: use selected and explicitly provided context first; call `list_files()` only when a broader metadata overview is needed.
- Keep full SDK details out of the default prompt; concise examples are usually enough. Add an on-demand SDK reference only if it does not cause unnecessary extra tool turns.
- Keep only minimal datatable rules in the base prompt:
  - use datatables for persistence;
  - call `list_datatables()` before schema work;
  - DDL must use `exec_datatable_sql`;
  - non-read SQL requires confirmation.

## Additional context assessment

The `@` context system is a good UX foundation.

App mode exposes categories for:

- frontend files;
- backend runnables;
- datatables.

Selecting a datatable context includes its columns and also calls `addTableToWhitelist(...)`, adding the table to the app data panel.

### Strengths

- Context is explicit and user-controllable.
- Datatable table selection is naturally integrated into the chat input.
- Selected app file/runnable chips are visible and can be excluded.
- Inspector and code-selection context are compact and useful.

### Concerns

1. `@` context persists across messages until manually removed, which can silently bloat follow-up prompts.
2. Available app context currently includes file contents/runnable configs in memory before selection.
3. Each selected context item is truncated, but there is no overall context budget indicator.
4. Current file/runnable selection is included by default unless excluded, which is convenient but not minimal.

### Recommendation

- Make app `@` context per-message by default.
- Add an explicit “pin” option for context that should persist across messages.
- Lazy-load file/runnable content when selected or when a message is sent.
- Show an approximate context-size/token budget indicator.
- Prefer sending path/name and selected code first; fetch full files only when necessary.

## Confirmation assessment

The generic confirmation mechanism already exists:

- `processToolCall(...)` checks `tool.requiresConfirmation`.
- `ToolExecutionDisplay.svelte` renders Run/Cancel controls.
- Script test runs, flow test runs, and mutating API calls already use confirmation.

App mode should use the same infrastructure for important actions.

### Suggested confirmation policy

#### No confirmation required

- `list_files`, as a metadata-only response;
- `get_frontend_file`;
- `get_backend_runnable`;
- `get_selected_context`;
- `list_datatables`, as table-name metadata only;
- `get_datatable_table_schema`, as a targeted schema read;
- `lint`;
- search tools.

#### Confirmation required

- `set_frontend_file`;
- `patch_file`;
- `delete_frontend_file`;
- `set_backend_runnable`;
- `delete_backend_runnable`;
- `exec_datatable_sql` for any DDL or DML;
- `exec_datatable_sql` for `SELECT` if it returns real row data that will be sent back to the model.

### Recommended UX

For files/runnables:

- Prefer batched proposed edits.
- Show a diff.
- Let the user click “Apply changes”.
- Run lint after applying.

For SQL:

- Show the exact SQL.
- Classify the query as:
  - schema read;
  - data read;
  - insert/update/delete;
  - DDL.
- Require confirmation before data reads and all mutations.
- For table creation, require both:
  - table creation policy enabled;
  - explicit confirmation of the `CREATE TABLE` SQL.

## Datatables integration assessment

The datatable integration is directionally good and already has several strong user-facing pieces.

### Current strengths

The new app setup lets the user choose:

- default datatable;
- schema mode: none, new, existing;
- whether AI can create tables;
- pre-whitelisted existing tables.

The raw app data panel lets users:

- add datatable table references;
- inspect tables through the DB manager drawer;
- configure the default datatable/schema for new tables.

The AI chat integration lets users:

- mention datatable tables through `@` context;
- add mentioned tables to the app whitelist;
- list datatable/schema/table names with `list_datatables()`;
- retrieve one table's columns with `get_datatable_table_schema()`;
- create tables through `exec_datatable_sql(..., new_table)`.

### Concerns

1. **`exec_datatable_sql` is too powerful without confirmation.**
   It can run `SELECT`, `INSERT`, `UPDATE`, `DELETE`, `CREATE`, `DROP`, `ALTER`, etc.

2. **Table creation policy is not fully enforced in code.**
   The tool blocks `new_table` when policy is disabled, but it does not block DDL if the model omits `new_table`.

3. **Table creation disabled state may not persist cleanly.**
   `RawAppData` stores `datatable` and `schema`, but not an explicit `enabled` value. `RawAppEditor` infers enabled from `data.datatable !== undefined`, which can re-enable table creation after reopening.

4. **Datatable context cache can become stale.**
   `AIChatManager.refreshDatatables()` runs when app helpers are set, but may not refresh immediately after data panel changes or after AI creates a new table.

5. **Full schema loading can still be too expensive internally.**
   `list_datatables()` and `get_datatable_table_schema()` reduce what is sent to the model, but they still currently rely on app helpers that fetch full schema data before filtering.

6. **Auto-whitelisting from `@table` is convenient but silent.**
   It mutates app data without an obvious confirmation or undo affordance.

### Recommended datatable tool design

Instead of one broad schema tool and one unrestricted SQL tool, prefer smaller tools:

- `list_datatables()`
- `list_datatable_tables(datatable, schema?, search?)` (optional backend/API optimization if table lists need server-side filtering)
- `get_datatable_table_schema(datatable, schema, table)`
- `preview_datatable_rows(datatable, schema, table, limit)` with confirmation
- `execute_datatable_sql(datatable, sql)` with query classification and confirmation
- `create_datatable_table(datatable, schema, table, columns)` as a structured safe path for table creation

## Priority recommendations

1. **Add confirmation to dangerous app tools**
   - file/runnable writes;
   - file/runnable deletes;
   - datatable SQL;
   - especially DDL/DML.

2. **Enforce SQL safety in code, not only in prompts**
   - block DDL unless `new_table` is provided and policy allows it;
   - confirm all non-`SELECT` statements;
   - consider confirming `SELECT` row reads too.

3. **Reduce default prompt/tool context**
   - keep `list_files()` metadata-only and demand-driven;
   - use selected context first;
   - keep full SDK references out of the default prompt;
   - keep datatable tools split into smaller schema/table lookups.

4. **Refresh datatable context reliably**
   - refresh after data panel changes;
   - refresh after `exec_datatable_sql(..., new_table)`;
   - remove debug logging from datatable refresh.

5. **Persist table creation policy explicitly**
   - store a boolean such as `tableCreationEnabled` in raw app data;
   - do not infer enabled solely from `data.datatable`.

6. **Improve `@` context lifecycle**
   - make app `@` context per-message by default;
   - add pinning for persistent context;
   - lazy-load file/runnable contents;
   - show approximate context size.

## Overall opinion

The current architecture is good and extensible, but it should become more demand-driven and safer before being considered efficient and user-safe.

The highest-impact changes are:

- add confirmation for app mutations and datatable SQL;
- enforce datatable SQL policy programmatically;
- reduce the app system prompt and avoid automatic broad context loading;
- split datatable schema access into smaller, targeted tools.
