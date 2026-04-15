## Special Modules

- Use `set_preprocessor_module` to add, replace, or remove the top-level `value.preprocessor_module`
- Use `set_failure_module` to add, replace, or remove the top-level `value.failure_module`
- Use `set_flow_json` only when you are replacing the whole flow, including normal modules and optional special modules

**Example - Update only the special modules:**
```javascript
set_preprocessor_module({
  module: JSON.stringify({
    id: "preprocessor",
    value: {
      type: "rawscript",
      language: "bun",
      content: "export async function preprocessor(payload: string) { const trimmed = payload.trim(); if (!trimmed) { throw new Error('payload must not be empty'); } return { payload: trimmed }; }",
      input_transforms: {
        payload: { type: "javascript", expr: "flow_input.payload" }
      }
    }
  })
})

set_failure_module({
  module: JSON.stringify({
    id: "failure",
    value: {
      type: "rawscript",
      language: "bun",
      content: "export async function main(message: string, name: string, step_id: string) { return { message, name, step_id }; }",
      input_transforms: {
        message: { type: "javascript", expr: "error.message" },
        name: { type: "javascript", expr: "error.name" },
        step_id: { type: "javascript", expr: "error.step_id" }
      }
    }
  })
})
```
