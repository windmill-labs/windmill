import type { StarterPrompt } from '$lib/onboardingProfile'

/** A prompt offered under the composer: `label` is the pill, `prompt` what it puts in the
 * box. `use` looks at, asks about or runs what exists; `build` creates or changes things. */
export type ComposerSuggestion = StarterPrompt & { mode: 'use' | 'build' }

/** What the reader may do where the session acts. An instance admin is not assumed to be an
 * admin of the workspace. */
export type ComposerRole = { operator: boolean; workspaceAdmin: boolean; instanceAdmin: boolean }

export type ComposerItem = { kind: string; path: string; name: string }

/** What the workspace and the instance look like now. A field is absent when its request
 * failed or the role may not send it, and an absent field fires no rule. */
export type ComposerSignals = {
	/** Capped at the page the list call reads. */
	itemCount?: number
	/** The reader's most recently updated draft. */
	myDraft?: ComposerItem
	/** The most recently updated item whose last deploy failed. */
	failedDeploy?: ComposerItem
	/** Runs that failed in the last 24 hours. */
	failedRuns?: number
	queue?: { waiting: number; suspended: number }
	hasErrorHandler?: boolean
	/** Workers that pinged in the last 5 minutes. */
	liveWorkers?: number
}

const BUILD_EXAMPLES: ComposerSuggestion[] = [
	{
		mode: 'build',
		label: 'Sync Salesforce',
		prompt: 'Sync new Salesforce leads into a postgres table every hour'
	},
	{
		mode: 'build',
		label: 'Weekly Slack report',
		prompt: 'Generate a weekly sales report from postgres and post it to Slack every Monday'
	},
	{
		mode: 'build',
		label: 'Ban Discord users',
		prompt:
			'Build a workflow that triggers on a Discord message, checks for offensive language using an LLM, and possibly block them'
	}
]

const MAX_SUGGESTIONS = 4
/** From this many jobs waiting, the queue reads as backed up. */
const BACKED_UP = 10
const MAX_NAME = 32

/** Up to four suggestions, most pressing first: an empty workspace's build examples, then
 * incidents, the reader's own unfinished work, admin setup, and fillers for the role. */
export function composerSuggestions(
	signals: ComposerSignals,
	role: ComposerRole
): ComposerSuggestion[] {
	const out: ComposerSuggestion[] = []
	const use = (label: string, prompt: string) => out.push({ mode: 'use', label, prompt })
	const build = (label: string, prompt: string) => out.push({ mode: 'build', label, prompt })
	const empty = signals.itemCount === 0

	if (empty && !role.operator) out.push(...BUILD_EXAMPLES)

	if (role.instanceAdmin && signals.liveWorkers === 0)
		use(
			"No worker has pinged in 5 minutes. What's wrong?",
			'No worker has pinged in the last 5 minutes, so no job can run. Find out what is wrong with the workers and how to bring them back.'
		)
	const failed = signals.failedRuns ?? 0
	if (failed > 0)
		use(
			failed === 1 ? 'Why did a run fail today?' : `Why did ${failed} runs fail today?`,
			'Which runs failed in this workspace in the last 24 hours, and why?'
		)
	const queue = signals.queue
	if (queue && (queue.waiting >= BACKED_UP || queue.suspended > 0)) {
		const waiting = queue.waiting + queue.suspended
		use(
			waiting === 1 ? 'Why is a job waiting?' : `Why are ${waiting} jobs waiting?`,
			`${waiting === 1 ? 'A job is' : `${waiting} jobs are`} waiting in this workspace's queue, ${queue.suspended} of them suspended. Why, and what would unblock them?`
		)
	}

	if (!role.operator) {
		const draft = signals.myDraft
		if (draft)
			build(
				`Finish my draft: ${shortName(draft.name)}`,
				`Help me finish my draft of the ${draft.kind} ${draft.path}`
			)
		const deploy = signals.failedDeploy
		if (deploy)
			build(
				`Fix the deploy of ${shortName(deploy.name)}`,
				`The last deploy of the ${deploy.kind} ${deploy.path} failed. Find out why and fix it.`
			)
	}

	// A workspace with nothing in it has nothing to report on or guard yet.
	if (!empty) {
		if (role.workspaceAdmin && signals.hasErrorHandler === false)
			build(
				'Set up an error handler for this workspace',
				'This workspace has no error handler. Help me set one up so that failed runs notify us.'
			)
		if (role.instanceAdmin)
			use(
				'Are the workers keeping up with the queue?',
				'Are the workers keeping up with the queue? Compare how long jobs wait before they start with how busy each worker group is.'
			)
		// Only questions the chat's tools can answer: it reads no critical alerts, and its run
		// listing has no date filter and stops at 100 runs, so "this week" totals are guesses.
		if (role.workspaceAdmin)
			use(
				'Which schedules keep failing?',
				'Which schedules in this workspace keep failing, and why?'
			)
		use(
			'How healthy were the last runs?',
			'Look at the last runs in this workspace: how many succeeded, which failed, and is anything getting slower?'
		)
	}

	return out.slice(0, MAX_SUGGESTIONS)
}

function shortName(name: string): string {
	return name.length > MAX_NAME ? `${name.slice(0, MAX_NAME - 1)}…` : name
}
