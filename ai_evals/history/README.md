Recorded history rows are anchored to the benchmark-definition commit used for the run.

That means `gitSha` points to the commit whose prompts, evaluators, and fixtures produced the recorded result. A later commit may only add the new JSONL row to git history without changing the benchmark itself.
