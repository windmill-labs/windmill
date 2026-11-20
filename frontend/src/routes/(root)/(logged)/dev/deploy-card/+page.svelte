<script lang="ts">
	// DEV-ONLY playground for the chat's deploy card: every deploy situation side by side, from
	// fixtures, to iterate on its design. Not linked anywhere; open at /dev/deploy-card. Only
	// reachable on dev builds.
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import DeployCardView, {
		type DeployCardReview
	} from '$lib/components/copilot/chat/DeployCardView.svelte'
	import type { DeployDiffSides } from '$lib/components/copilot/chat/DeployDiffButton.svelte'
	import type { DeployCardIdentity } from '$lib/components/copilot/chat/DeployCardView.svelte'
	import type { InheritedAccess } from '$lib/components/itemSettings/inheritedAccess'
	import { workspaceStore } from '$lib/stores'

	type Scenario = {
		id: string
		title: string
		type: string
		path: string
		review: DeployCardReview
		/** Set by the Path field of a first deploy. */
		deployPath?: string
		publicAccess?: 'anonymous' | 'guest'
		/** `u/name`s: the draft's user, the deployed one, the folder default. */
		runAs?: { held?: string; saved?: string; folderDefault?: string }
		deploymentMessage?: string
		force?: boolean
		diff?: DeployDiffSides
	}

	const ME = 'u/admin'

	const user = (name: string | undefined) => ({
		email: name ? `${name.slice(2)}@windmill.dev` : undefined,
		permissionedAs: name
	})
	function identityFor(s: Scenario): DeployCardIdentity | undefined {
		if (s.type !== 'script' && s.type !== 'flow' && s.type !== 'app') return undefined
		return {
			kind: s.type,
			held: user(s.runAs?.held),
			saved: user(s.runAs?.saved),
			folderDefault: s.runAs?.folderDefault,
			rawApp: s.type === 'app'
		}
	}

	function script(content: string, extra: Record<string, unknown> = {}) {
		return {
			summary: 'Nightly export',
			language: 'bun',
			content,
			schema: { type: 'object', properties: {}, required: [] },
			...extra
		}
	}
	const v1 = 'export async function main() {\n  return "export v1"\n}\n'
	const v2 =
		'export async function main() {\n  const rows = await fetchRows()\n  return { count: rows.length, version: "v2" }\n}\n'
	const scriptDiff: DeployDiffSides = { kind: 'script', deployed: script(v1), draft: script(v2) }
	const newDiff: DeployDiffSides = { kind: 'script', deployed: undefined, draft: script(v2) }

	const folderAccess: InheritedAccess = {
		source: 'Folder reports',
		rows: [
			{ name: 'u/admin', write: true },
			{ name: 'g/all', write: false },
			{ name: 'u/bob', write: false }
		]
	}
	// Who reaches a path, as `inheritedAccess` would answer for these made-up folders.
	function accessFor(path: string): InheritedAccess {
		const [root, name] = path.split('/')
		if (root === 'u') return { source: 'Owner', rows: [{ name: `u/${name}`, write: true }] }
		if (name === 'reports') return folderAccess
		if (name === 'secret') return { source: 'Folder secret', rows: [], note: 'Members of folder secret' }
		return {
			source: `Folder ${name}`,
			rows: [
				{ name: 'u/admin', write: true },
				{ name: `g/${name}_team`, write: false }
			]
		}
	}

	const ready = (added: number, removed: number): DeployCardReview => ({
		state: 'ready',
		isNew: false,
		added,
		removed
	})
	const fresh: DeployCardReview = { state: 'ready', isNew: true, added: 0, removed: 0 }

	const scenarios: Scenario[] = $state([
		{
			id: 'update-you',
			title: 'Update, runs as you',
			type: 'script',
			path: 'u/admin/nightly_export',
			review: ready(3, 1),
			runAs: { held: ME, saved: ME },
			deploymentMessage: 'Count the exported rows',
			diff: scriptDiff
		},
		{
			id: 'update-plain',
			title: 'Update, runs as its caller, no message',
			type: 'flow',
			path: 'f/reports/daily_digest',
			review: ready(12, 4),
			diff: {
				kind: 'flow',
				deployed: { summary: 'a', value: { modules: [] } },
				draft: { summary: 'b', value: { modules: [] } }
			}
		},
		{
			id: 'update-other',
			title: 'Update, keeps running as someone else',
			type: 'script',
			path: 'u/admin/nightly_export',
			review: ready(1, 1),
			runAs: { held: 'u/bob', saved: 'u/bob' },
			deploymentMessage: 'v2',
			diff: scriptDiff
		},
		{
			id: 'update-takes-over',
			title: 'Update, run-as switches from bob to you',
			type: 'script',
			path: 'f/billing/charge_customers',
			review: ready(2, 2),
			runAs: { held: ME, saved: 'u/bob' },
			deploymentMessage: 'Retry failed charges once',
			diff: scriptDiff
		},
		{
			id: 'update-stops',
			title: 'Update, stops running as someone',
			type: 'flow',
			path: 'f/billing/monthly_invoices',
			review: ready(0, 1),
			runAs: { saved: 'u/bob' },
			diff: scriptDiff
		},
		{
			id: 'update-no-change',
			title: 'Update with no content change',
			type: 'script',
			path: 'u/admin/nightly_export',
			review: ready(0, 0),
			diff: { kind: 'script', deployed: script(v1), draft: script(v1) }
		},
		{
			id: 'update-force',
			title: 'Update that overwrites a newer deploy',
			type: 'script',
			path: 'f/reports/weekly_summary',
			review: ready(5, 2),
			runAs: { held: ME, saved: ME },
			deploymentMessage: 'Fix the date range',
			force: true,
			diff: scriptDiff
		},
		{
			id: 'new-folder',
			title: 'First deploy into a folder',
			type: 'script',
			path: 'f/reports/weekly_summary',
			review: fresh,
			diff: newDiff
		},
		{
			id: 'new-folder-default',
			title: 'First deploy, folder runs new items as a default user',
			type: 'flow',
			path: 'f/reports/weekly_summary',
			review: fresh,
			runAs: { folderDefault: 'u/reports_bot' },
			deploymentMessage: 'First version',
			diff: newDiff
		},
		{
			id: 'new-folder-unlisted',
			title: 'First deploy, folder members cannot be listed',
			type: 'script',
			path: 'f/secret/rotate_keys',
			review: fresh,
			diff: newDiff
		},
		{
			id: 'new-mine',
			title: 'First deploy under your user',
			type: 'script',
			path: 'u/admin/scratch',
			review: fresh,
			diff: newDiff
		},
		{
			id: 'new-other-user',
			title: "First deploy under someone else's user",
			type: 'script',
			path: 'u/bob/helper',
			review: fresh,
			diff: newDiff
		},
		{
			id: 'new-app',
			title: 'First deploy of an app',
			type: 'app',
			path: 'f/reports/dashboard',
			review: fresh,
			runAs: { held: ME },
			diff: { kind: 'raw_app', deployed: undefined, draft: { summary: 'Dashboard' } }
		},
		{
			id: 'new-app-anonymous',
			title: 'First deploy of a public app',
			type: 'app',
			path: 'u/admin/status_page',
			review: fresh,
			publicAccess: 'anonymous',
			runAs: { held: ME },
			diff: { kind: 'raw_app', deployed: undefined, draft: { summary: 'Status' } }
		},
		{
			id: 'new-app-guest',
			title: 'First deploy of a guest app',
			type: 'app',
			path: 'f/support/ticket_form',
			review: fresh,
			publicAccess: 'guest',
			runAs: { held: 'u/bob' },
			deploymentMessage: 'Open the form to customers',
			diff: { kind: 'raw_app', deployed: undefined, draft: { summary: 'Tickets' } }
		},
		{
			id: 'trigger',
			title: 'Trigger (no run-as, no access row)',
			type: 'trigger',
			path: 'f/reports/on_new_order',
			review: ready(2, 1),
			diff: { kind: 'script', deployed: { path: 'a' }, draft: { path: 'b' } }
		},
		{
			id: 'long',
			title: 'Long path and message',
			type: 'script',
			path: 'f/a_rather_long_folder_name/with_a_very_long_script_name_that_keeps_going',
			review: ready(240, 198),
			runAs: { held: 'u/someone_with_a_long_username', saved: 'u/another_long_username' },
			deploymentMessage:
				'Rewrite the export to stream rows in batches of 500 instead of loading the whole table, and retry each batch up to three times',
			diff: scriptDiff
		},
		{
			id: 'loading',
			title: 'Reading the draft',
			type: 'script',
			path: 'u/admin/nightly_export',
			review: { state: 'loading' },
			deploymentMessage: 'v2'
		},
		{
			id: 'error',
			title: 'Draft could not be read',
			type: 'script',
			path: 'u/admin/nightly_export',
			review: { state: 'error', message: 'You have no draft at u/admin/nightly_export' },
			deploymentMessage: 'v2'
		}
	])

	// The chat panel's usual widths: docked narrow, docked wide, full page.
	let width = $state('420')
</script>

{#if import.meta.env.DEV}
	<div class="min-h-screen bg-surface-secondary p-6 flex flex-col gap-4">
		<div class="flex items-center gap-4">
			<h1 class="text-lg font-semibold text-emphasis whitespace-nowrap">Deploy card</h1>
			<ToggleButtonGroup bind:selected={width}>
				{#snippet children({ item })}
					<ToggleButton value="340" label="340px" {item} />
					<ToggleButton value="420" label="420px" {item} />
					<ToggleButton value="640" label="640px" {item} />
				{/snippet}
			</ToggleButtonGroup>
			<span class="text-xs text-tertiary"
				>Deploy and cancel do nothing here; Diff opens fixtures.</span
			>
		</div>
		<div class="flex flex-wrap gap-6 items-start">
			{#each scenarios as s (s.id)}
				<section class="flex flex-col gap-1.5" style="width: {width}px" id={s.id}>
					<h2 class="text-xs font-medium text-secondary">{s.title}</h2>
					<div class="bg-surface rounded-md p-2">
						<DeployCardView
							toolCallId={undefined}
							workspace={$workspaceStore}
							type={s.type}
							path={s.path}
							pathKind={s.type === 'app' ? 'app' : s.type === 'script' || s.type === 'flow' ? s.type : undefined}
							bind:deployPath={s.deployPath}
							review={s.review}
							me={ME}
							access={s.review.state === 'ready' && s.review.isNew
								? accessFor(s.deployPath ?? s.path)
								: undefined}
							appAccess={s.type === 'app' && s.review.state === 'ready' && s.review.isNew
								? (s.publicAccess ?? 'publisher')
								: undefined}
							identity={identityFor(s)}
							deploymentMessage={s.deploymentMessage}
							force={s.force}
							loadDiff={s.diff ? async () => s.diff! : undefined}
						/>
					</div>
				</section>
			{/each}
		</div>
	</div>
{/if}
