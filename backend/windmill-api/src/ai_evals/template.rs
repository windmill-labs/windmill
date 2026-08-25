use super::*;

/// What a script scorer starts from.
pub const SCORER_SCRIPT_TEMPLATE: &str = r#"// A scorer receives one run and returns a number between 0 and 1, a boolean, or
// { score, reason, checks } — checks show up in the case detail.
// Return { score: null } for a case this scorer has nothing to measure on: the cell
// is left out of the column's mean and pass rate rather than counted as a zero.
//
// The run is also handed to you spelled out, so a short scorer can skip the type below
// entirely: export async function main(output: unknown, expected: unknown) { ... }
type ToolCall = {
  name: string
  args?: Record<string, unknown>
  result?: unknown
  error?: string
  duration_ms?: number
  truncated?: boolean
}

type EvalRun = {
  input: { user_message?: string; user_attachments?: unknown[] }
  output?: unknown
  expected?: unknown
  tool_calls: ToolCall[]
  tools: { name: string; schema?: Record<string, unknown> }[]
  metrics: { steps: number; duration_ms?: number; usage?: Record<string, unknown> }
  status: string
  job_id: string
}

export async function main(run: EvalRun) {
  // How the agent got to its answer. Reported rather than scored: checks render in the case
  // detail either way, so they explain the number without being averaged into it.
  const checks = [
    check('arguments match the schema', args_schema_valid(run)),
    check('no repeated calls', no_repeated_calls(run)),
    check('no failed tool calls', no_step_errors(run)),
    check('under 6 steps', run.metrics.steps <= 6, `${run.metrics.steps} steps`),
    check('under 30 seconds', under_ms(run, 30_000), `${run.metrics.duration_ms ?? '?'} ms`)
  ]

  // Nothing to compare the answer against, so this column has no verdict on this case rather
  // than a failing one. The cell reads n/a and the column's mean is of the cases it measured.
  if (run.expected == undefined) {
    return { score: null, reason: 'this case has no expected answer', checks }
  }

  // One question per column, and this column's question is whether the answer is right.
  // Deliberately not the share of checks above that passed: a right answer that was slow and a
  // wrong answer that was fast would score the same, and the column could not say which it was.
  const correct = contains(run.output, text(run.expected))
  return {
    score: correct ? 1 : 0,
    reason: correct ? undefined : `expected ${text(run.expected)}`,
    checks
  }
}

// Helpers. Edit or delete freely.

function check(name: string, passed: boolean, detail?: string) {
  return { name, passed, detail }
}

function text(value: unknown): string {
  return typeof value === 'string' ? value : JSON.stringify(value ?? '')
}

function contains(output: unknown, needle: string): boolean {
  return needle.trim().length > 0 && text(output).toLowerCase().includes(needle.trim().toLowerCase())
}

// Every call validated against the schema of the tool it called. A tool whose schema could not be
// resolved is not checked rather than failed.
function args_schema_valid(run: EvalRun): boolean {
  return run.tool_calls.every((call) => {
    const schema = run.tools.find((tool) => tool.name === call.name)?.schema as
      | { properties?: Record<string, { type?: string }>; required?: string[] }
      | undefined
    if (!schema?.properties) return true
    const args = call.args ?? {}
    for (const key of schema.required ?? []) {
      if (args[key] === undefined || args[key] === null) return false
    }
    for (const [key, value] of Object.entries(args)) {
      const expected = schema.properties[key]?.type
      if (!expected) continue
      const actual = Array.isArray(value) ? 'array' : value === null ? 'null' : typeof value
      if (expected === 'integer' ? !Number.isInteger(value) : expected !== actual) return false
    }
    return true
  })
}

// The same tool called twice with the same arguments.
function no_repeated_calls(run: EvalRun): boolean {
  const seen = new Set<string>()
  for (const call of run.tool_calls) {
    const key = `${call.name}:${JSON.stringify(call.args ?? {})}`
    if (seen.has(key)) return false
    seen.add(key)
  }
  return true
}

function no_step_errors(run: EvalRun): boolean {
  return run.status === 'success' && run.tool_calls.every((call) => !call.error)
}

// A run with no recorded duration is not under the limit: a check that could not be evaluated
// should not report as one that passed.
function under_ms(run: EvalRun, max: number): boolean {
  const ms = run.metrics.duration_ms
  return ms != undefined && ms <= max
}
"#;

#[derive(Serialize)]
pub struct ScorerDefaults {
    /// The system prompt a judge agent is created with. It lives on that agent afterwards.
    pub judge_prompt: String,
    /// The starting point for a script scorer, held here so the shape a scorer is handed and the
    /// template that reads it cannot drift apart.
    pub script_template: String,
}

pub async fn scorer_defaults() -> JsonResult<ScorerDefaults> {
    Ok(Json(ScorerDefaults {
        judge_prompt: JUDGE_SYSTEM_PROMPT.to_string(),
        script_template: SCORER_SCRIPT_TEMPLATE.to_string(),
    }))
}
