<script lang="ts">
	import WorkspaceDiffDrawer from '$lib/components/sessions/WorkspaceDiffDrawer.svelte'
	import { Button } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import DarkModeToggle from '$lib/components/sidebar/DarkModeToggle.svelte'
	import type { DeployItem } from '$lib/components/sessions/sessionDeployModel'
	import type {
		DeploymentStatus,
		SessionDeployModel
	} from '$lib/components/sessions/sessionDeployModel.svelte'

	// Playground for the session Edits drawer deploy animation: the REAL
	// WorkspaceDiffDrawer driven by a mock model with adjustable latency,
	// outcome, data-flip lag, and content size — no backend involved.

	let deployMs = $state(800)
	let flipDelayMs = $state(300)
	let holdMs = $state(1200)
	let diffLines = $state(25)
	let itemCount = $state(3)
	let failDeploys = $state(false)
	let permOk = $state(true)

	function makeItem(i: number, done = false): DeployItem {
		return {
			key: `script:u/dev/demo_${i}`,
			deployKind: 'script',
			draftKind: 'script',
			path: `u/dev/demo_${i}`,
			displayPath: `u/dev/demo_${i}`,
			summary: `Demo script ${i}`,
			done,
			hasDraft: !done,
			draftOnly: i % 2 === 0,
			draftUsers: undefined,
			mine: true,
			canWrite: true,
			legacy: false,
			rawApp: false
		}
	}

	let items = $state<DeployItem[]>([])
	let statuses = $state<Record<string, DeploymentStatus>>({})
	let deploying = $state(false)

	function reset() {
		statuses = {}
		items = [...Array.from({ length: itemCount }, (_, i) => makeItem(i)), makeItem(99, true)]
	}
	reset()

	const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))

	function genContent(seed: string, lines: number, mutated: boolean): string {
		const out = [`// ${seed}`, 'export async function main() {']
		for (let i = 0; i < lines; i++) {
			out.push(
				mutated && i % 4 === 0
					? `\tconst v${i} = ${i} * 2 // changed by the chat`
					: `\tconst v${i} = ${i}`
			)
		}
		if (mutated) out.push('\tconst added = "new trailing logic"')
		out.push(`\treturn v0`, '}')
		return out.join('\n')
	}

	function fakeScript(content: string) {
		return { summary: 'Demo', description: '', content, language: 'bun', schema: {} }
	}

	function setStatus(key: string, s: DeploymentStatus | undefined) {
		const next = { ...statuses }
		if (s) next[key] = s
		else delete next[key]
		statuses = next
	}

	const model: SessionDeployModel = {
		get items() {
			return items
		},
		get loading() {
			return false
		},
		get error() {
			return undefined
		},
		load() {},
		async loadDiffValues(item: DeployItem) {
			await sleep(250)
			const after = fakeScript(genContent(item.key, diffLines, true))
			if (item.done) return { before: after, after }
			return {
				before: item.draftOnly ? undefined : fakeScript(genContent(item.key, diffLines, false)),
				after
			}
		},
		get deploying() {
			return deploying
		},
		statusOf(key: string) {
			return statuses[key]
		},
		get deployPermission() {
			return permOk
				? { ok: true as const }
				: { ok: false as const, reason: 'Deploy disabled by the playground toggle' }
		},
		async deployRow(item: DeployItem) {
			setStatus(item.key, { status: 'loading' })
			deploying = true
			await sleep(deployMs)
			deploying = false
			if (failDeploys) {
				setStatus(item.key, { status: 'failed', error: 'Simulated failure from the playground' })
				return false
			}
			setStatus(item.key, undefined)
			// Mimic the production lag between deploy success and the draft
			// refetch flipping the item to done.
			setTimeout(() => {
				items = items.map((it) =>
					it.key === item.key ? { ...it, done: true, hasDraft: false, draftOnly: false } : it
				)
			}, flipDelayMs)
			return true
		},
		async discardRow(item: DeployItem) {
			await sleep(deployMs)
			items = items.filter((it) => it.key !== item.key)
		}
	}

	let drawer: WorkspaceDiffDrawer | undefined = $state(undefined)
</script>

<div class="p-8 flex flex-col gap-6 max-w-2xl">
	<div class="flex items-center justify-between">
		<h1 class="text-lg font-semibold text-primary">Deploy animation playground</h1>
		<DarkModeToggle forcedDarkMode={false} />
	</div>
	<p class="text-xs text-secondary">
		Drives the real <code>WorkspaceDiffDrawer</code> with a mock deploy model. Tune the knobs, open the
		drawer, hit Deploy, feel the transition. Reset re-arms the drafts.
	</p>

	<div class="grid grid-cols-2 gap-x-8 gap-y-4 text-xs text-primary">
		<label class="flex flex-col gap-1">
			<span>Deploy latency: <b>{deployMs}ms</b> (spinner phase)</span>
			<input type="range" min="0" max="5000" step="100" bind:value={deployMs} />
		</label>
		<label class="flex flex-col gap-1">
			<span>Success hold: <b>{holdMs}ms</b> (green check beat)</span>
			<input type="range" min="0" max="4000" step="100" bind:value={holdMs} />
		</label>
		<label class="flex flex-col gap-1">
			<span>Data flip lag: <b>{flipDelayMs}ms</b> (mock draft refetch)</span>
			<input type="range" min="0" max="3000" step="100" bind:value={flipDelayMs} />
		</label>
		<label class="flex flex-col gap-1">
			<span>Diff size: <b>{diffLines}</b> lines</span>
			<input type="range" min="3" max="200" step="1" bind:value={diffLines} />
		</label>
		<label class="flex flex-col gap-1">
			<span>Items: <b>{itemCount}</b> drafts (+1 deployed)</span>
			<input type="range" min="1" max="10" step="1" bind:value={itemCount} onchange={reset} />
		</label>
		<div class="flex flex-col gap-2 justify-center">
			<Toggle bind:checked={failDeploys} options={{ right: 'Fail deploys' }} size="xs" />
			<Toggle bind:checked={permOk} options={{ right: 'Deploy permission' }} size="xs" />
		</div>
	</div>

	<div class="flex items-center gap-2">
		<Button variant="accent" unifiedSize="sm" onclick={() => drawer?.open()}>Open drawer</Button>
		<Button variant="default" unifiedSize="sm" onclick={reset}>Reset items</Button>
	</div>
</div>

<WorkspaceDiffDrawer
	bind:this={drawer}
	{model}
	title="Deploy animation playground"
	workspaceLabel="playground-workspace"
	successHoldMs={holdMs}
/>
