# Pull Request Plans for Windmill Repository

This document outlines detailed plans for two PRs to improve the Windmill CLI and MCP server functionality.

---

## PR #1: Fix wmill CLI Authentication with Workspace-Scoped Tokens

### Problem Statement

The `wmill workspace add` command fails when using workspace-scoped tokens because it attempts to verify workspace existence using the `/api/workspaces/exists` endpoint, which requires permissions that workspace-scoped tokens don't have.

**Current Behavior:**
```bash
wmill workspace add local admins http://localhost:8666 --token <workspace-token>
# Error: Unauthorized - Not authorized: Unauthorized
```

**Root Cause:**
In `cli/src/commands/workspace/workspace.ts` (line 200-202), the code calls:
```typescript
alreadyExists = await wmill.existsWorkspace({
  requestBody: { id: workspaceId },
});
```

This endpoint (`/api/workspaces/exists`) uses `user_db.begin(&authed)` in the backend, which requires the user to have access to at least one workspace through workspace-scoped authentication. However, the workspace being checked may not be one the user has access to yet, causing the authentication to fail.

**Why This is a Problem:**
- Users with workspace-scoped tokens cannot use `wmill workspace add`
- Forces users to create super admin tokens just to configure the CLI
- Breaks the principle of least privilege
- Makes CI/CD integration harder (workspace tokens are preferred for automation)

### Proposed Solution

Modify the `add` function in `cli/src/commands/workspace/workspace.ts` to gracefully handle authentication failures when checking workspace existence.

**Strategy:**
Instead of treating the `existsWorkspace` failure as a fatal error, catch authentication errors and skip the workspace existence check when using workspace-scoped tokens.

### Implementation Details

#### File: `cli/src/commands/workspace/workspace.ts`

**Change the `add` function** (lines 194-242):

**Current code:**
```typescript
setClient(
  token,
  remote.endsWith("/") ? remote.substring(0, remote.length - 1) : remote
);
let alreadyExists = false;
try {
  alreadyExists = await wmill.existsWorkspace({
    requestBody: { id: workspaceId },
  });
} catch (e) {
  log.info(
    colors.red.bold("! Credentials or instance is invalid. Aborting.")
  );
  throw e;
}
if (opts.create) {
  if (!alreadyExists) {
    log.info(
      colors.yellow(
        `Workspace at id ${workspaceId} on ${remote} does not exist. Creating...`
      )
    );
    // ... create workspace code ...
  }
} else if (!alreadyExists) {
  log.info(
    colors.red.bold(
      `! Workspace at id ${workspaceId} on ${remote} does not exist. Re-run with --create to create it. Aborting.`
    )
  );
  // ... list workspaces code ...
  Deno.exit(1);
}
```

**Proposed code:**
```typescript
setClient(
  token,
  remote.endsWith("/") ? remote.substring(0, remote.length - 1) : remote
);

// Try to verify workspace existence, but don't fail if using workspace-scoped token
let alreadyExists: boolean | undefined = undefined;
let canVerifyWorkspace = true;

try {
  alreadyExists = await wmill.existsWorkspace({
    requestBody: { id: workspaceId },
  });
} catch (e) {
  // Check if this is an authentication error (likely workspace-scoped token)
  const errorMsg = e instanceof Error ? e.message : String(e);
  if (errorMsg.includes("Unauthorized") || errorMsg.includes("401")) {
    log.info(
      colors.yellow(
        "⚠ Cannot verify workspace existence (workspace-scoped token detected)."
      )
    );
    log.info(
      colors.yellow(
        "  The CLI will attempt to add the workspace anyway."
      )
    );
    canVerifyWorkspace = false;
  } else {
    // For other errors (network, etc), abort
    log.info(
      colors.red.bold("! Could not connect to instance. Aborting.")
    );
    throw e;
  }
}

// Handle workspace creation
if (opts.create) {
  if (alreadyExists === false) {
    log.info(
      colors.yellow(
        `Workspace at id ${workspaceId} on ${remote} does not exist. Creating...`
      )
    );
    const automateUsernameCreation: boolean =
      ((await wmill.getGlobal({
        key: "automate_username_creation",
      })) as any) ?? false;
    await wmill.createWorkspace({
      requestBody: {
        id: workspaceId,
        name: opts.createWorkspaceName ?? workspaceName,
        username: automateUsernameCreation ? undefined : opts.createUsername,
      },
    });
  } else if (alreadyExists === true) {
    log.info(
      colors.green(
        `Workspace ${workspaceId} already exists on ${remote}.`
      )
    );
  }
  // If alreadyExists is undefined (couldn't verify), proceed anyway
} else if (alreadyExists === false) {
  // Workspace definitely doesn't exist and --create not specified
  log.info(
    colors.red.bold(
      `! Workspace at id ${workspaceId} on ${remote} does not exist. Re-run with --create to create it. Aborting.`
    )
  );
  log.info(
    "On that instance and with those credentials, the workspaces that you can access are:"
  );
  try {
    const workspaces = await wmill.listWorkspaces();
    for (const workspace of workspaces) {
      log.info(`- ${workspace.id} (name: ${workspace.name})`);
    }
  } catch (e) {
    log.info(colors.yellow("  (Unable to list workspaces with provided token)"));
  }
  Deno.exit(1);
} else if (!canVerifyWorkspace) {
  // Couldn't verify workspace existence, but user didn't specify --create
  log.info(
    colors.yellow(
      `⚠ Could not verify if workspace ${workspaceId} exists on ${remote}.`
    )
  );
  log.info(
    colors.yellow(
      "  If the workspace doesn't exist, operations will fail. Use --create to create it if needed."
    )
  );
}
```

### Testing Plan

1. **Test with super admin token** (existing functionality):
   ```bash
   wmill workspace add test test_workspace http://localhost:8666 --token <super-admin-token>
   # Should work as before
   ```

2. **Test with workspace-scoped token** (new functionality):
   ```bash
   wmill workspace add local admins http://localhost:8666 --token <workspace-token>
   # Should succeed with warning about workspace verification
   ```

3. **Test with invalid token**:
   ```bash
   wmill workspace add test test http://localhost:8666 --token invalid
   # Should fail appropriately
   ```

4. **Test with --create flag**:
   ```bash
   wmill workspace add new new_ws http://localhost:8666 --token <token> --create
   # Should work with both token types
   ```

5. **Test workspace operations after adding**:
   ```bash
   wmill workspace add local admins http://localhost:8666 --token <workspace-token>
   wmill script list  # Should work
   wmill flow list    # Should work
   ```

### Alternative Solution (Backend Fix)

An alternative approach would be to fix the backend endpoint to allow workspace existence checks with workspace-scoped tokens:

**File:** `backend/windmill-api/src/workspaces.rs`

**Current code (line 455-470):**
```rust
async fn exists_workspace(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Json(WorkspaceId { id }): Json<WorkspaceId>,
) -> JsonResult<bool> {
    let mut tx = user_db.begin(&authed).await?;
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM workspace WHERE workspace.id = $1)",
        id
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);
    tx.commit().await?;
    Ok(Json(exists))
}
```

**Proposed code:**
```rust
async fn exists_workspace(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,  // Use raw DB instead of UserDB
    Json(WorkspaceId { id }): Json<WorkspaceId>,
) -> JsonResult<bool> {
    let mut tx = db.begin().await?;

    // Check if workspace exists AND user has access to it
    let exists_and_has_access = sqlx::query_scalar!(
        "SELECT EXISTS(
            SELECT 1 FROM workspace w
            JOIN usr u ON u.workspace_id = w.id
            WHERE w.id = $1 AND u.email = $2
        )",
        id,
        authed.email
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);

    tx.commit().await?;
    Ok(Json(exists_and_has_access))
}
```

**Pros of backend fix:**
- More secure (only returns true if user has access)
- Works for all clients, not just CLI
- Doesn't change API semantics (still checks existence + access)

**Cons:**
- Changes semantic meaning of "exists" (now means "exists AND I have access")
- May break other code that relies on checking arbitrary workspace existence
- Requires backend deployment

**Recommendation:** Implement the CLI fix first (no backend changes needed), then optionally implement the backend fix as a follow-up PR.

### Documentation Updates

Add to `cli/README.md`:

```markdown
### Workspace-Scoped Tokens

When using workspace-scoped tokens (created within a specific workspace), the CLI may not be able to verify workspace existence. This is expected behavior. The CLI will add the workspace configuration anyway and warn you about the limitation.

If you need to verify workspace existence or create new workspaces, use a super admin token instead.

**Example with workspace-scoped token:**
```bash
# This will work, but with a warning
wmill workspace add local admins http://localhost:8666 --token <workspace-token>

# Operations will work normally after adding
wmill script list
wmill flow get f/my/flow
```
```

### Summary

**Files Changed:**
- `cli/src/commands/workspace/workspace.ts` (1 function modified)
- `cli/README.md` (documentation added)

**Lines of Code:** ~40 lines changed

**Breaking Changes:** None

**Benefits:**
- ✅ Workspace-scoped tokens now work with `wmill workspace add`
- ✅ No breaking changes to existing functionality
- ✅ Better error messages and user guidance
- ✅ Follows principle of least privilege
- ✅ Easier CI/CD integration

---

## PR #2: Add Flow Mutation Operations to MCP Server

### Problem Statement

The Windmill MCP server currently exposes read-only flow operations (`listFlows`, `getFlowByPath`) but doesn't expose flow creation, update, or deletion operations. This limits the ability to use AI assistants (like Claude) to programmatically manage flows.

**Current MCP Operations:**
- ✅ `listFlows` - List all flows
- ✅ `getFlowByPath` - Get a specific flow
- ❌ `createFlow` - Not exposed
- ❌ `updateFlow` - Not exposed
- ❌ `deleteFlowByPath` - Not exposed
- ❌ `archiveFlowByPath` - Not exposed

**Why This is a Problem:**
- Users can't create or modify flows through Claude or other AI assistants
- Forces manual updates or workarounds (like running preview scripts)
- Inconsistent with other MCP operations (variables, resources, schedules all have full CRUD)
- Limits usefulness of AI-assisted workflow development

### Proposed Solution

Mark flow mutation endpoints in the OpenAPI specification with `x-mcp-tool: true`, then regenerate the MCP tools.

### Implementation Details

#### File: `backend/windmill-api/openapi.yaml`

**1. Enable `createFlow` endpoint** (line 6558):

```yaml
  /w/{workspace}/flows/create:
    post:
      summary: create flow
      operationId: createFlow
      x-mcp-tool: true  # ADD THIS LINE
      x-mcp-instructions: "Create a new flow. The request body must include 'path' (the flow identifier, e.g., 'f/examples/my_flow'), 'summary', 'value' (the flow definition as a JSON object with 'modules' array), and optionally 'schema' (JSON Schema for flow inputs). Set 'draft_only' to true to create a draft without deploying."  # ADD THIS LINE
      tags:
        - flow
      parameters:
        - $ref: "#/components/parameters/WorkspaceId"
      requestBody:
        description: Partially filled flow
        required: true
        content:
          application/json:
            schema:
              allOf:
                - $ref: "#/components/schemas/OpenFlowWPath"
                - type: object
                  properties:
                    draft_only:
                      type: boolean
                    deployment_message:
                      type: string
      responses:
        "201":
          description: flow created
          content:
            text/plain:
              schema:
                type: string
```

**2. Enable `updateFlow` endpoint** (line 6588):

```yaml
  /w/{workspace}/flows/update/{path}:
    post:
      summary: update flow
      operationId: updateFlow
      x-mcp-tool: true  # ADD THIS LINE
      x-mcp-instructions: "Update an existing flow. The path parameter identifies the flow to update (e.g., 'f/examples/my_flow'). The request body should include 'value' (updated flow definition), and optionally 'summary', 'description', 'schema', and 'deployment_message'. This creates a new version of the flow."  # ADD THIS LINE
      tags:
        - flow
      parameters:
        - $ref: "#/components/parameters/WorkspaceId"
        - $ref: "#/components/parameters/ScriptPath"
      requestBody:
        description: Partially filled flow
        required: true
        content:
          application/json:
            schema:
              allOf:
                - $ref: "#/components/schemas/OpenFlowWPath"
                - type: object
                  properties:
                    deployment_message:
                      type: string
      responses:
        "200":
          description: flow updated
          content:
            text/plain:
              schema:
                type: string
```

**3. Enable `archiveFlowByPath` endpoint** (line 6618):

```yaml
  /w/{workspace}/flows/archive/{path}:
    post:
      summary: archive flow by path
      operationId: archiveFlowByPath
      x-mcp-tool: true  # ADD THIS LINE
      x-mcp-instructions: "Archive or unarchive a flow. Set 'archived' to true to archive (soft delete) the flow, or false to unarchive it. Archived flows are hidden from listings but can be restored."  # ADD THIS LINE
      tags:
        - flow
      parameters:
        - $ref: "#/components/parameters/WorkspaceId"
        - $ref: "#/components/parameters/ScriptPath"
      requestBody:
        description: archiveFlow
        required: true
        content:
          application/json:
            schema:
              type: object
              properties:
                archived:
                  type: boolean
      responses:
        "200":
          description: flow archived
          content:
            text/plain:
              schema:
                type: string
```

**4. Enable `deleteFlowByPath` endpoint** (line 6645):

```yaml
  /w/{workspace}/flows/delete/{path}:
    delete:
      summary: delete flow by path
      operationId: deleteFlowByPath
      x-mcp-tool: true  # ADD THIS LINE
      x-mcp-instructions: "Permanently delete a flow. This operation cannot be undone. The path parameter identifies the flow (e.g., 'f/examples/my_flow'). Use 'keep_captures=true' query parameter to preserve captured webhook/trigger data."  # ADD THIS LINE
      tags:
        - flow
      parameters:
        - $ref: "#/components/parameters/WorkspaceId"
        - $ref: "#/components/parameters/ScriptPath"
        - name: keep_captures
          description: keep captures
          in: query
          schema:
            type: boolean
      responses:
        "200":
          description: flow deleted
          content:
            text/plain:
              schema:
                type: string
```

#### File: `backend/generate_mcp_endpoints_tools/generate_mcp_tools.py`

No changes needed - the script already handles these endpoint types correctly.

#### Regeneration Steps

After modifying the OpenAPI spec, run:

```bash
cd backend/generate_mcp_endpoints_tools
pip install -r requirements.txt
python3 generate_mcp_tools.py
```

This will regenerate:
- `backend/windmill-api/src/mcp/tools/auto_generated_endpoints.rs`
- `frontend/src/lib/mcpEndpointTools.ts`

#### Commit these generated files

```bash
git add backend/windmill-api/openapi.yaml
git add backend/windmill-api/src/mcp/tools/auto_generated_endpoints.rs
git add frontend/src/lib/mcpEndpointTools.ts
```

### Testing Plan

1. **Regenerate MCP tools:**
   ```bash
   cd backend/generate_mcp_endpoints_tools
   python3 generate_mcp_tools.py
   # Verify output shows 4 new tools added
   ```

2. **Build and test backend:**
   ```bash
   cd backend
   cargo build
   # Verify build succeeds with new MCP tools
   ```

3. **Test with MCP client:**
   ```typescript
   // List available tools
   const tools = await mcp.listTools();
   console.log(tools.filter(t => t.name.includes('Flow')));
   // Should show: createFlow, updateFlow, archiveFlowByPath, deleteFlowByPath

   // Test createFlow
   await mcp.callTool('createFlow', {
     workspace: 'admins',
     body: {
       path: 'f/test/example_flow',
       summary: 'Test flow',
       value: {
         modules: [
           {
             id: 'a',
             value: {
               type: 'rawscript',
               content: 'export async function main() { return "Hello"; }',
               language: 'deno'
             }
           }
         ]
       }
     }
   });

   // Test updateFlow
   await mcp.callTool('updateFlow', {
     workspace: 'admins',
     path: 'f/test/example_flow',
     body: {
       summary: 'Updated test flow',
       deployment_message: 'Updated via MCP'
     }
   });

   // Test archiveFlowByPath
   await mcp.callTool('archiveFlowByPath', {
     workspace: 'admins',
     path: 'f/test/example_flow',
     body: { archived: true }
   });

   // Test deleteFlowByPath
   await mcp.callTool('deleteFlowByPath', {
     workspace: 'admins',
     path: 'f/test/example_flow',
     query: { keep_captures: false }
   });
   ```

4. **Test with Claude Code (real-world usage):**
   ```
   User: "Create a flow that runs a Python script to analyze data"
   Claude: [Uses createFlow MCP tool]

   User: "Update the flow to add error handling"
   Claude: [Uses updateFlow MCP tool]

   User: "Archive that flow"
   Claude: [Uses archiveFlowByPath MCP tool]
   ```

### Schema Validation

Verify the `OpenFlowWPath` schema is correctly defined in the OpenAPI spec:

```bash
grep -A 50 "OpenFlowWPath:" backend/windmill-api/openapi.yaml
```

The schema should include:
- `path` (string, required)
- `summary` (string)
- `description` (string)
- `value` (object, the flow definition)
- `schema` (object, JSON Schema for inputs)

### Documentation Updates

Add to `backend/generate_mcp_endpoints_tools/README.md`:

```markdown
### Available Flow Operations

The MCP server exposes the following flow operations:

- `listFlows` - List all flows in workspace
- `getFlowByPath` - Get details of a specific flow
- `createFlow` - Create a new flow
- `updateFlow` - Update an existing flow
- `archiveFlowByPath` - Archive/unarchive a flow
- `deleteFlowByPath` - Permanently delete a flow

### Flow Creation Example

```json
{
  "tool": "createFlow",
  "arguments": {
    "workspace": "my_workspace",
    "body": {
      "path": "f/examples/my_flow",
      "summary": "My example flow",
      "value": {
        "modules": [
          {
            "id": "a",
            "value": {
              "type": "rawscript",
              "content": "export async function main() { return 'Hello'; }",
              "language": "deno"
            }
          }
        ]
      },
      "schema": {
        "type": "object",
        "properties": {},
        "required": []
      }
    }
  }
}
```
```

Add to `frontend/src/lib/mcpEndpointTools.ts` (auto-generated, but document):

```typescript
/**
 * Auto-generated MCP tools from OpenAPI specification.
 *
 * Flow operations:
 * - createFlow: Create a new flow
 * - updateFlow: Update an existing flow
 * - archiveFlowByPath: Archive or unarchive a flow
 * - deleteFlowByPath: Permanently delete a flow
 *
 * To regenerate: cd backend/generate_mcp_endpoints_tools && python3 generate_mcp_tools.py
 */
```

### Comparison with Other Resources

Verify consistency with other resource types:

| Resource | List | Get | Create | Update | Delete | MCP Status |
|----------|------|-----|--------|--------|--------|------------|
| **Variables** | ✅ | ✅ | ✅ | ✅ | ✅ | Full CRUD |
| **Resources** | ✅ | ✅ | ✅ | ✅ | ✅ | Full CRUD |
| **Schedules** | ✅ | ✅ | ✅ | ✅ | ✅ | Full CRUD |
| **Scripts** | ✅ | ✅ | ❌ | ❌ | ❌ | Read-only |
| **Flows** (before) | ✅ | ✅ | ❌ | ❌ | ❌ | Read-only |
| **Flows** (after) | ✅ | ✅ | ✅ | ✅ | ✅ | Full CRUD ✨ |

### Potential Concerns & Mitigations

**Concern:** Flow creation/update is complex and might fail with incomplete data

**Mitigation:**
- Add comprehensive `x-mcp-instructions` to guide AI models
- Include examples in documentation
- The OpenAPI schema validation will catch invalid requests
- Error messages from the API will guide corrections

**Concern:** Destructive operations (delete) might be dangerous

**Mitigation:**
- Archive operation is safer (soft delete) and should be preferred
- Delete requires explicit confirmation in instructions
- MCP context is workspace-scoped, limiting blast radius
- All operations respect Windmill's existing RBAC

**Concern:** Scripts don't have create/update, why should flows?

**Response:**
- Scripts and flows are different: flows are JSON-based and more amenable to programmatic generation
- Scripts require code generation which is better done via `runScriptPreview`
- This PR focuses on flows; scripts could be a follow-up if needed

### Summary

**Files Changed:**
- `backend/windmill-api/openapi.yaml` (4 endpoints modified)
- `backend/windmill-api/src/mcp/tools/auto_generated_endpoints.rs` (auto-generated)
- `frontend/src/lib/mcpEndpointTools.ts` (auto-generated)
- `backend/generate_mcp_endpoints_tools/README.md` (documentation)

**Lines of Code:** ~8 lines added to OpenAPI, ~200 lines auto-generated

**Breaking Changes:** None (additive only)

**Benefits:**
- ✅ Full flow CRUD through MCP
- ✅ AI-assisted flow development
- ✅ Consistent with other resource types
- ✅ No backend code changes needed
- ✅ Automatically documented through OpenAPI
- ✅ Type-safe (schema validated)

---

## Implementation Order

### Recommended Sequence

1. **PR #2 (MCP Server)** - Implement first
   - Simpler change (just OpenAPI annotations)
   - No breaking changes
   - Faster to review and merge
   - Provides immediate value for AI-assisted development

2. **PR #1 (wmill CLI)** - Implement second
   - More complex change (logic modification)
   - Requires more thorough testing
   - Depends on understanding authentication model
   - Can reference MCP pattern for consistency

### Combined Testing

After both PRs are merged, test the full workflow:

```bash
# Set up CLI with workspace-scoped token (PR #1)
wmill workspace add local admins http://localhost:8666 --token <workspace-token>

# Use Claude with MCP to create a flow (PR #2)
# Claude: "Create a flow that processes user data"
# [Creates flow via MCP]

# Verify with CLI
wmill flow list
wmill flow get f/examples/user_processor
```

---

## Questions for Maintainers

### PR #1 (CLI)

1. **Preferred approach:** CLI-side fix (proposed) or backend endpoint fix (alternative)?
2. **Error handling:** Should we skip workspace verification silently or warn the user?
3. **Testing:** Any specific test cases or edge cases to cover?

### PR #2 (MCP)

1. **Script operations:** Should we also enable script create/update via MCP in the same PR?
2. **Safety:** Any additional safety checks needed for destructive operations?
3. **Documentation:** Should we add inline examples to the OpenAPI spec?

---

## Timeline Estimate

### PR #1 (wmill CLI)
- Implementation: 2-3 hours
- Testing: 2 hours
- Documentation: 1 hour
- Review cycles: 1-2 days
- **Total: 3-5 days**

### PR #2 (MCP Server)
- Implementation: 1 hour
- Regeneration: 15 minutes
- Testing: 2 hours
- Documentation: 1 hour
- Review cycles: 1-2 days
- **Total: 2-3 days**

### Both PRs
- **Combined: 5-8 days** (with some parallelization possible)

---

## Success Criteria

### PR #1 Success Criteria
- [ ] Workspace-scoped tokens work with `wmill workspace add`
- [ ] Super admin tokens still work (no regression)
- [ ] Clear error messages for network/auth failures
- [ ] Documentation updated
- [ ] Tests passing

### PR #2 Success Criteria
- [ ] All 4 flow operations exposed via MCP
- [ ] MCP tools regenerate successfully
- [ ] Backend compiles with new tools
- [ ] Frontend integrates new tools
- [ ] Documentation updated
- [ ] Example usage tested with Claude

### Integration Success Criteria
- [ ] Full workflow works: CLI setup → MCP flow creation → CLI verification
- [ ] No breaking changes to existing functionality
- [ ] Improved user experience for both issues

---

## Additional Notes

### Related Issues

Check if there are existing GitHub issues for these problems:
- Search for "wmill workspace add token"
- Search for "MCP flow create"
- Search for "workspace-scoped token"

### Community Impact

These PRs will:
- Improve developer experience for CLI users
- Enable AI-assisted workflow development
- Reduce friction in CI/CD pipelines
- Follow security best practices (least privilege)
- Make Windmill more accessible to AI-powered tools

### Future Enhancements

After these PRs, consider:
- Script create/update via MCP
- App create/update via MCP
- Better MCP documentation with examples
- CLI command to export MCP tool list
- Integration tests for CLI + MCP workflows
