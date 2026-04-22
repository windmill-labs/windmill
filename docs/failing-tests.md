# Failing Tests

This file tracks benchmark cases that still fail or need follow-up validation.

## Flow

- `flow-test6-ai-agent-tools`
  Latest failing run: `ai_evals/results/2026-04-09T11-25-24.107Z__flow`
  Issues:
  final output does not include the actions or tool-result details the prompt asks for
  `open_support_ticket` contains a syntax bug

- `flow-test7-simple-modification`
  Latest failing run: `ai_evals/results/2026-04-09T11-25-24.107Z__flow`
  Issues:
  `validate_data` was added, but the failure behavior still does not match the requested contract
  `save_results` throws instead of returning a graceful structured result

- `flow-test11-preprocessor-and-failure-handler`
  Latest failing run: `ai_evals/results/2026-04-09T11-25-24.107Z__flow`
  Issues:
  the model creates regular `preprocessor` and `failure` modules
  it does not use Windmill's special top-level `preprocessor_module` and `failure_module`

## Needs Reconfirmation

- `flow-test4-order-processing-loop`
  Full-suite failing run: `ai_evals/results/2026-04-09T11-25-24.107Z__flow`
  Follow-up passing run after prompt improvement: `ai_evals/results/2026-04-09T13-29-15.877Z__flow`
  Note:
  this case failed on invalid `branchone` downstream result access
  it passed after adding explicit branch-output guidance to the flow prompt
  rerun the full flow suite to confirm the fix holds in the broader benchmark
