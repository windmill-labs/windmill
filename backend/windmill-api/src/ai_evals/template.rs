use super::*;

/// What a script scorer starts from. The assertions are in `main` and the helpers below it, so
/// the file reads as the checks that were chosen rather than as a library to learn.

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
  //
  // tool_called(run, ['search']) and tool_not_called(run, ['refund']) assert which tools a case
  // should and should not reach for. under_tokens(run, 20_000) and cost_under(run, 0.05, rate)
  // bound what a run may spend.
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
  //
  // Other ways to judge an answer: exact_match(run.output, run.expected) where it must match
  // exactly, json_equals for a structured answer whose key order does not matter, and
  // matches(run.output, /^ORD-\d+$/) for a shape rather than a value.
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

function exact_match(output: unknown, expected: unknown): boolean {
  return text(output).trim() === text(expected).trim()
}

// Key order and whitespace insensitive.
function json_equals(a: unknown, b: unknown): boolean {
  const sort = (value: unknown): unknown =>
    Array.isArray(value)
      ? value.map(sort)
      : value && typeof value === 'object'
        ? Object.fromEntries(
            Object.entries(value as Record<string, unknown>)
              .sort(([x], [y]) => x.localeCompare(y))
              .map(([k, v]) => [k, sort(v)])
          )
        : value
  return JSON.stringify(sort(a)) === JSON.stringify(sort(b))
}

function contains(output: unknown, needle: string): boolean {
  return needle.trim().length > 0 && text(output).toLowerCase().includes(needle.trim().toLowerCase())
}

function matches(output: unknown, re: RegExp): boolean {
  return re.test(text(output))
}

// Allow list: every named tool was called.
function tool_called(run: EvalRun, names: string[]): boolean {
  return names.every((name) => run.tool_calls.some((call) => call.name === name))
}

// Deny list: none of the named tools was called.
function tool_not_called(run: EvalRun, names: string[]): boolean {
  return !run.tool_calls.some((call) => names.includes(call.name))
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

// Same rule for usage: a provider that reported no token counts has not stayed under a budget.
function tokens_used(run: EvalRun): number | undefined {
  const usage = (run.metrics.usage ?? {}) as Record<string, number | undefined>
  const input = usage.input_tokens
  const output = usage.output_tokens
  if (input == undefined && output == undefined) return undefined
  return (input ?? 0) + (output ?? 0)
}

function under_tokens(run: EvalRun, max: number): boolean {
  const tokens = tokens_used(run)
  return tokens != undefined && tokens <= max
}

// Windmill keeps no provider price table, so the rate is yours to set: dollars per 1k tokens.
function cost_under(run: EvalRun, usd: number, rate: { input: number; output: number }): boolean {
  if (tokens_used(run) == undefined) return false
  const usage = (run.metrics.usage ?? {}) as Record<string, number | undefined>
  const cost =
    ((usage.input_tokens ?? 0) / 1000) * rate.input +
    ((usage.output_tokens ?? 0) / 1000) * rate.output
  return cost <= usd
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

// -----------------------------------------------------------------------------------------------
// Capturing a case from real traffic
// -----------------------------------------------------------------------------------------------
