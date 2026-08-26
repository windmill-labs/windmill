<script lang="ts">
	import { Button } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import FlowPathViewer from '$lib/components/flows/content/FlowPathViewer.svelte'
	import { FlowService, type OpenFlow } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { untrack } from 'svelte'
	import { fixtureFlow, subFixtureFlow } from './fixtureFlow'

	// FlowPathViewer takes a path and fetches, so the fixture has to exist in the workspace.
	// Seeding on load keeps fixtureFlow.ts the single source of truth for the graph.
	let path = $derived(`u/${$userStore?.username ?? 'admin'}/dev_flow_path_viewer`)

	let seeded = $state<string | undefined>(undefined)
	let seeding = $state(false)
	let error = $state<string | undefined>(undefined)

	let noSide = $state(false)
	let fillAvailableHeight = $state(true)
	let frame = $state<'full' | 'panel' | 'short'>('full')

	// Inline styles rather than Tailwind classes: the arbitrary sizes are the point of this page
	// and JIT only emits the ones it happened to scan.
	const frameStyle = {
		full: 'height: 100%; width: 100%',
		panel: 'height: 520px; width: 55%',
		short: 'height: 260px; width: 100%'
	}

	async function upsert(workspace: string, p: string, flow: OpenFlow) {
		const body = { path: p, ...flow, deployment_message: 'dev fixture' }
		if (await FlowService.existsFlowByPath({ workspace, path: p })) {
			await FlowService.updateFlow({ workspace, path: p, requestBody: body })
		} else {
			await FlowService.createFlow({ workspace, requestBody: body })
		}
	}

	async function seed(workspace: string, p: string) {
		seeding = true
		error = undefined
		try {
			// Subflow first: the main fixture's step 'm' points at it, and a step whose target
			// does not exist renders as not-found instead of a nested graph.
			await upsert(workspace, `${p}_sub`, subFixtureFlow)
			await upsert(workspace, p, fixtureFlow(`${p}_sub`))
			seeded = undefined
			await new Promise((r) => setTimeout(r, 0))
			seeded = p
		} catch (e: any) {
			error = e?.body ?? e?.message ?? String(e)
			sendUserToast(`Could not seed the fixture flow: ${error}`, true)
		} finally {
			seeding = false
		}
	}

	$effect(() => {
		const workspace = $workspaceStore
		const p = path
		if (!workspace || !$userStore) return
		// seed() writes seeded/seeding, so the guard has to read them outside the dependency set
		untrack(() => {
			if (seeded !== p && !seeding) seed(workspace, p)
		})
	})
</script>

<div class="h-screen w-full flex flex-col min-h-0">
	<div class="flex flex-wrap items-center gap-4 border-b px-4 py-2 text-xs bg-surface-secondary">
		<span class="font-semibold">FlowPathViewer</span>
		<span class="text-tertiary font-mono">{path}</span>

		<Toggle bind:checked={noSide} size="xs" options={{ right: 'noSide' }} />
		<Toggle
			bind:checked={fillAvailableHeight}
			size="xs"
			options={{ right: 'fillAvailableHeight' }}
		/>

		<ToggleButtonGroup bind:selected={frame} noWFull>
			{#snippet children({ item })}
				<ToggleButton value="full" label="Full page" {item} />
				<ToggleButton value="panel" label="Raw-app pane (55%)" {item} />
				<ToggleButton value="short" label="Short (260px)" {item} />
			{/snippet}
		</ToggleButtonGroup>

		<Button
			unifiedSize="xs"
			variant="default"
			loading={seeding}
			onclick={() => $workspaceStore && seed($workspaceStore, path)}
		>
			Re-seed
		</Button>
	</div>

	<div class="grow min-h-0 p-4">
		<div class="border rounded min-h-0 flex flex-col overflow-hidden" style={frameStyle[frame]}>
			{#if error}
				<div class="p-4 text-xs text-red-600 font-mono whitespace-pre-wrap">{error}</div>
			{:else if seeded}
				{#key seeded}
					<FlowPathViewer path={seeded} {noSide} {fillAvailableHeight} />
				{/key}
			{:else}
				<div class="p-4 text-xs text-tertiary">Seeding the fixture flow…</div>
			{/if}
		</div>
	</div>
</div>
