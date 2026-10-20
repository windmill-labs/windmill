import type { StarterPrompt } from '$lib/onboardingProfile'

/** A prompt offered under the composer: `label` is the pill, `prompt` what it puts in the
 * box. `use` looks at, asks about or runs what exists; `build` creates or changes things. */
export type ComposerSuggestion = StarterPrompt & { mode: 'use' | 'build' }

/** What the reader may do where the session acts. An instance admin is not assumed to be an
 * admin of the workspace. */
export type ComposerRole = { operator: boolean; workspaceAdmin: boolean; instanceAdmin: boolean }

export type ComposerItem = { kind: string; path: string; name: string }

/** A connection the workspace holds; `type` is its resource type. */
export type ComposerResource = { type: string; path: string; name: string }

/** Below this many items the workspace is new enough that building leads. */
export const FEW_ITEMS = 10

/** The databases whose tables the chat can read (`get_db_schema`), by resource type. */
export const DATABASE_TYPES: Record<string, string> = {
	postgresql: 'Postgres',
	mysql: 'MySQL',
	ms_sql_server: 'SQL Server',
	snowflake: 'Snowflake',
	bigquery: 'BigQuery'
}

/** Where a job can send a message, by resource type, most preferred first. */
export const CHANNEL_TYPES: Record<string, { to: string; send: string }> = {
	slack: { to: 'to Slack', send: 'post it to Slack' },
	ms_teams_webhook: { to: 'to Teams', send: 'post it to Microsoft Teams' },
	discord_webhook: { to: 'to Discord', send: 'post it to Discord' },
	telegram: { to: 'to Telegram', send: 'send it on Telegram' },
	gmail: { to: 'by email', send: 'email it' },
	smtp: { to: 'by email', send: 'email it' }
}

/** What the workspace looks like now: read from the composer's item and resource lists, plus
 * the reader's favorites. An absent field fires no rule. */
export type ComposerSignals = {
	/** Capped at the page the list call reads. */
	itemCount?: number
	/** The reader's draft among the items the list call reads, the most recently updated. */
	myDraft?: ComposerItem
	/** Among the same items, the most recently updated one whose last deploy failed. */
	failedDeploy?: ComposerItem
	/** Among the same items, the most recently updated flow outside the reader's user folder. */
	sharedFlow?: ComposerItem
	/** A script or flow the reader starred. */
	favorite?: ComposerItem
	/** The most recently edited database connection. */
	database?: ComposerResource
	/** The connection a job would send a message through, the most preferred type first. */
	channel?: ComposerResource
}

/** Each example shows a different piece of Windmill: a schedule, an approval step, a webhook. */
const BUILD_EXAMPLES: ComposerSuggestion[] = [
	{
		mode: 'build',
		label: 'Daily metrics to Slack',
		prompt:
			"Every weekday at 9am, query postgres for yesterday's signups and revenue and post a summary to the #metrics Slack channel"
	},
	{
		mode: 'build',
		label: 'Approve refunds in Slack',
		prompt:
			'Build a flow that asks for approval in Slack before issuing a refund in Stripe, and only refunds if someone approves'
	},
	{
		mode: 'build',
		label: 'Triage tickets to Slack',
		prompt:
			'When a support ticket arrives by webhook, classify its urgency with an LLM and post the urgent ones to the #support Slack channel'
	}
]

const MAX_SUGGESTIONS = 4
const MAX_NAME = 32

/** Up to four suggestions, most pressing first: the reader's own unfinished work, then pills
 * naming what the workspace holds, the build examples while it is empty, and checks for the
 * role. A prompt names the exact item or connection path: the chat cannot list resources by
 * type, so "our database" would leave it guessing. The checks claim nothing about the
 * workspace, so they read the same whether or not anything is wrong. */
export function composerSuggestions(
	signals: ComposerSignals,
	role: ComposerRole
): ComposerSuggestion[] {
	const out: ComposerSuggestion[] = []
	const use = (label: string, prompt: string) => out.push({ mode: 'use', label, prompt })
	const build = (label: string, prompt: string) => out.push({ mode: 'build', label, prompt })
	const empty = signals.itemCount === 0
	const few = signals.itemCount !== undefined && signals.itemCount < FEW_ITEMS
	const { database, channel } = signals
	// Operators can neither build nor read a database's tables.
	const builder = !role.operator

	if (builder) {
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

	const summary = few && builder && database && channel ? { database, channel } : undefined
	const dailySummary = () => {
		if (!summary) return
		const db = summary.database
		const send = CHANNEL_TYPES[summary.channel.type]
		build(
			`Daily ${shortName(db.name)} summary ${send.to}`,
			`Every weekday at 9am, summarize yesterday's activity in the ${DATABASE_TYPES[db.type]} database ${db.path} and ${send.send} with the resource ${summary.channel.path}`
		)
	}
	const explore = () => {
		if (!builder || !database) return
		use(
			`What's in ${shortName(database.name)}?`,
			`Look at the tables in the ${DATABASE_TYPES[database.type]} database ${database.path} and tell me what data it holds.`
		)
	}
	const walkThrough = () => {
		const flow = signals.sharedFlow
		if (!flow) return
		use(
			`Walk me through ${shortName(flow.name)}`,
			`Walk me through what the flow ${flow.path} does, step by step.`
		)
	}
	const run = () => {
		const favorite = signals.favorite
		if (!favorite) return
		use(`Run ${shortName(favorite.name)}`, `Run the ${favorite.kind} ${favorite.path} for me.`)
	}

	// Building leads while the workspace is new; once it holds work, what is there leads. An
	// operator mostly runs what others built.
	if (few) {
		dailySummary()
		explore()
	}
	if (role.operator) {
		run()
		walkThrough()
	} else {
		walkThrough()
		run()
	}
	if (!few) explore()

	if (empty && builder) {
		const [dailyMetrics, ...others] = BUILD_EXAMPLES
		out.push(...(summary ? others : [dailyMetrics, ...others]))
	}

	// A workspace with nothing in it has nothing to check yet. Keep these to what the chat's
	// tools can answer: its run listing has no date filter and stops at 100 runs, so a prompt
	// naming a period ("today", "this week") gets a guess.
	if (!empty) {
		if (role.instanceAdmin)
			use(
				'Check worker health',
				'Are the workers healthy and keeping up with the queue? Compare how long jobs wait before they start with how busy each worker group is.'
			)
		if (role.workspaceAdmin)
			use(
				'Check the schedules',
				'Do any schedules in this workspace keep failing? If so, which ones and why?'
			)
		use(
			'Check recent runs',
			'Look at the recent runs in this workspace: did any fail, and why? Is anything getting slower?'
		)
		use(
			'Check the run queue',
			"Is anything stuck in this workspace's queue? Which jobs are waiting or suspended, and why?"
		)
	}

	return out.slice(0, MAX_SUGGESTIONS)
}

function shortName(name: string): string {
	return name.length > MAX_NAME ? `${name.slice(0, MAX_NAME - 1)}…` : name
}
