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
export const CHANNEL_TYPES: Record<
	string,
	{ name: string; to: string; send: string; message: string }
> = {
	slack: {
		name: 'Slack',
		to: 'to Slack',
		send: 'post it to Slack',
		message: 'post a Slack message'
	},
	ms_teams_webhook: {
		name: 'Microsoft Teams',
		to: 'to Teams',
		send: 'post it to Microsoft Teams',
		message: 'post a Teams message'
	},
	discord_webhook: {
		name: 'Discord',
		to: 'to Discord',
		send: 'post it to Discord',
		message: 'post a Discord message'
	},
	telegram: {
		name: 'Telegram',
		to: 'to Telegram',
		send: 'send it on Telegram',
		message: 'send a Telegram message'
	},
	gmail: { name: 'Gmail', to: 'by email', send: 'email it', message: 'send an email' },
	smtp: { name: 'email', to: 'by email', send: 'email it', message: 'send an email' }
}

/** What the workspace looks like now: read from the composer's item, resource and data table
 * lists, plus the reader's favorites. An absent field fires no rule. A `null` channel and an
 * empty `dataTables` mean the list answered and holds none, which fires the pills that teach
 * the missing piece. */
export type ComposerSignals = {
	/** Capped at the page the list call reads. */
	itemCount?: number
	/** The reader's draft among the items the list call reads, the most recently updated. */
	myDraft?: ComposerItem
	/** Among the same items the reader may edit, the most recently updated one whose last
	 * deploy failed. */
	failedDeploy?: ComposerItem
	/** Among the same items, the most recently updated flow outside the reader's user folder. */
	sharedFlow?: ComposerItem
	/** Among the same items, the most recently updated deployed script with a main function. */
	script?: ComposerItem
	/** Among the same items the reader may edit, the most recently updated full-code app. */
	app?: ComposerItem
	/** Among the same items, the most recently updated AI agent. */
	agent?: ComposerItem
	/** Among the same items, the most recently updated one in a folder. */
	folderItem?: ComposerItem
	/** A script or flow the reader starred. */
	favorite?: ComposerItem
	/** The most recently edited database connection. */
	database?: ComposerResource
	/** The connection a job would send a message through, the most preferred type first. */
	channel?: ComposerResource | null
	/** The workspace's data tables, `main` first. */
	dataTables?: string[]
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

/** Up to four suggestions that each teach one Windmill concept on the reader's own workspace,
 * most pressing first: their unfinished work, the next piece the workspace is missing, then
 * what it already holds, the build examples while it is empty, and checks for the role. A
 * concept is offered once. A prompt names the exact item or connection path: the chat cannot
 * list resources by type, so "our database" would leave it guessing. The checks claim nothing
 * about the workspace, so they read the same whether or not anything is wrong. */
export function composerSuggestions(
	signals: ComposerSignals,
	role: ComposerRole
): ComposerSuggestion[] {
	const out: ComposerSuggestion[] = []
	const taught = new Set<string>()
	const add = (
		concept: string,
		mode: ComposerSuggestion['mode'],
		label: string,
		prompt: string
	) => {
		if (taught.has(concept)) return
		taught.add(concept)
		out.push({ mode, label, prompt })
	}
	const empty = signals.itemCount === 0
	const few = signals.itemCount !== undefined && signals.itemCount < FEW_ITEMS
	const { database, channel, dataTables, script, sharedFlow } = signals
	const dataTable = dataTables?.[0]
	// Operators run what others built and author nothing.
	const builder = !role.operator

	if (builder) {
		const draft = signals.myDraft
		if (draft)
			add(
				'draft',
				'build',
				`Finish my draft: ${shortName(draft.name)}`,
				`Help me finish my draft of the ${draft.kind} ${draft.path}`
			)
		const deploy = signals.failedDeploy
		if (deploy)
			add(
				'deploy',
				'build',
				`Fix the deploy of ${shortName(deploy.name)}`,
				`The last deploy of the ${deploy.kind} ${deploy.path} failed. Find out why and fix it.`
			)

		// The next piece the workspace is missing, one at a time: where data lives, then a
		// channel to send messages through. Past its first items a workspace may keep its data
		// in its own database and reach Slack through workspace settings, neither of which these
		// lists see, so only the admin's data table setup is offered there.
		if (dataTables?.length === 0) {
			// Only an admin can set up a data table.
			if (role.workspaceAdmin)
				add(
					'next',
					'build',
					'Set up a data table',
					'This workspace has no data table yet. Walk me through setting one up in the workspace settings, then show me how a script writes to it.'
				)
			else if (few)
				add(
					'next',
					'use',
					'How do I store data?',
					'How should my scripts store data in Windmill? Explain data tables and show me how a script reads and writes one.'
				)
		}
		if (channel === null && few)
			add(
				'next',
				'build',
				'Connect Slack',
				'Help me connect Slack to this workspace so my scripts can post messages.'
			)
	}

	const send = channel ? CHANNEL_TYPES[channel.type] : undefined
	const summary = () => {
		if (!few || !builder || !database || !channel || !send) return
		add(
			'summary',
			'build',
			`Daily ${shortName(database.name)} summary ${send.to}`,
			`Every weekday at 9am, summarize yesterday's activity in the ${DATABASE_TYPES[database.type]} database ${database.path} and ${send.send} with the resource ${channel.path}`
		)
	}
	const exploreTable = () => {
		if (!dataTable) return
		add(
			'datatable',
			'use',
			`What's in the ${shortName(dataTable)} data table?`,
			`List the tables in the data table ${dataTable} and tell me what each one holds.`
		)
	}
	const walkThrough = () => {
		const flow = sharedFlow
		if (flow)
			add(
				'walk',
				'use',
				`Walk me through ${shortName(flow.name)}`,
				`Walk me through what the flow ${flow.path} does, step by step.`
			)
		const agent = signals.agent
		if (agent)
			add(
				'walk',
				'use',
				`Walk me through ${shortName(agent.name)}`,
				`Walk me through what the AI agent ${agent.path} does and which tools it can use.`
			)
	}
	const run = () => {
		const favorite = signals.favorite
		if (favorite)
			add(
				'run',
				'use',
				`Run ${shortName(favorite.name)}`,
				`Run the ${favorite.kind} ${favorite.path} for me.`
			)
	}
	const access = () => {
		const item = signals.folderItem
		if (item)
			add(
				'access',
				'use',
				`Who can see ${shortName(item.name)}?`,
				`Who can see and edit the ${item.kind} ${item.path}, and why?`
			)
	}

	if (role.operator) {
		run()
		walkThrough()
		exploreTable()
		if (!empty)
			add(
				'operator',
				'use',
				'What can I run here?',
				'Which scripts and flows can I run in this workspace, and what does each one do?'
			)
		access()
	} else {
		summary()
		exploreTable()
		walkThrough()
		run()
		const app = signals.app
		if (app)
			add(
				'app',
				'build',
				`Add a page to ${shortName(app.name)}`,
				`Add a page to the app ${app.path}, and ask me what it should show.`
			)
		else if (dataTable)
			add(
				'app',
				'build',
				`Build an app on the ${shortName(dataTable)} data table`,
				`Build a full-code app to browse and edit the rows of a table in the data table ${dataTable}.`
			)
		if (channel && send)
			add(
				'channel',
				'use',
				`${capitalize(send.message)} from a script`,
				`Show me how a script uses the resource ${channel.path} to ${send.message}, with a short example.`
			)
		if (database?.type === 'postgresql')
			add(
				'postgres',
				'build',
				`React to new rows in ${shortName(database.name)}`,
				`Run a script whenever a row is inserted into a table of the Postgres database ${database.path}, and explain how Postgres triggers work.`
			)
		if (script) {
			add(
				'endpoint',
				'build',
				`Make ${shortName(script.name)} an API endpoint`,
				`Expose the script ${script.path} as an HTTP API endpoint that requires a Windmill token, and show me how to call it.`
			)
			if (few)
				add(
					'schedule',
					'build',
					`Run ${shortName(script.name)} every morning`,
					`Run the script ${script.path} every weekday at 9am. Ask me for any input it needs, and explain how schedules work.`
				)
		}
		if (sharedFlow)
			add(
				'webhook',
				'use',
				`Call ${shortName(sharedFlow.name)} by webhook`,
				`How do I start the flow ${sharedFlow.path} from another app with its webhook? Show me the URL and an example request.`
			)
		access()
		// While the workspace is new; an empty one needs its build examples more.
		if (few && !empty)
			add(
				'secret',
				'build',
				'Store an API key as a secret',
				'I have an API key my scripts need. Store it as a secret and show me how a script reads it with wmill.getVariable.'
			)
		if (channel && send)
			add(
				'hub',
				'use',
				`Find Hub scripts for ${send.name}`,
				`Find scripts on the Windmill Hub for ${send.name} that I could use with the resource ${channel.path}.`
			)
	}

	if (empty && builder) {
		const [dailyMetrics, ...others] = BUILD_EXAMPLES
		out.push(...(taught.has('summary') ? others : [dailyMetrics, ...others]))
	}

	// A workspace with nothing in it has nothing to check yet. Keep these to what the chat's
	// tools can answer: its run listing has no date filter and stops at 100 runs, so a prompt
	// naming a period ("today", "this week") gets a guess.
	if (!empty) {
		if (role.instanceAdmin)
			add(
				'workers',
				'use',
				'Check worker health',
				'Are the workers healthy and keeping up with the queue? Compare how long jobs wait before they start with how busy each worker group is.'
			)
		if (role.workspaceAdmin)
			add(
				'schedules',
				'use',
				'Check the schedules',
				'Do any schedules in this workspace keep failing? If so, which ones and why?'
			)
		add(
			'runs',
			'use',
			'Check recent runs',
			'Look at the recent runs in this workspace: did any fail, and why? Is anything getting slower?'
		)
		add(
			'queue',
			'use',
			'Check the run queue',
			"Is anything stuck in this workspace's queue? Which jobs are waiting or suspended, and why?"
		)
	}

	if (builder && !empty)
		add(
			'mcp',
			'use',
			'Use my scripts from Claude',
			'How do I connect Claude to this workspace with MCP so it can run my scripts and flows?'
		)

	return out.slice(0, MAX_SUGGESTIONS)
}

function shortName(name: string): string {
	return name.length > MAX_NAME ? `${name.slice(0, MAX_NAME - 1)}…` : name
}

function capitalize(text: string): string {
	return text.charAt(0).toUpperCase() + text.slice(1)
}
