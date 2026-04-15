## Special Modules

If the flow needs preprocessing before the main modules or a dedicated failure handler, prefer the dedicated tools:

- Use `set_preprocessor_module` for the special top-level preprocessor module with id `preprocessor`
- Use `set_failure_module` for the special top-level failure module with id `failure`
- Do NOT put `preprocessor` or `failure` inside the regular `modules` array

**Example - Add preprocessor and failure handler:**
```javascript
set_flow_json({
  modules: [
    {
      id: "process_event",
      summary: "Process the event payload",
      value: {
        type: "rawscript",
        language: "bun",
        content: "export async function main(payload: string) { return { success: true, payload }; }",
        input_transforms: {
          payload: { type: "javascript", expr: "flow_input.payload" }
        }
      }
    }
  ],
  schema: {
    type: "object",
    properties: {
      payload: { type: "string" }
    },
    required: ["payload"]
  }
})

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
