export const FREE_EXECUTION_QUOTA = 1000

/** Executions each paid seat includes per month (mirrors the billing page). */
export const SEAT_EXECUTION_QUOTA = 10000

export const EXECUTIONS_HINT =
	'An execution is one second of compute, not one job run: a job counts as 1 execution, plus 1 more for each additional second it runs.' as const
