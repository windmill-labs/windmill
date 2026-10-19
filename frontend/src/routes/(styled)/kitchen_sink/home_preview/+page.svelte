<script lang="ts">
	import { onMount } from 'svelte'
	import { RotateCw, SquareArrowOutUpRight } from 'lucide-svelte'
	import { WorkspaceService } from '$lib/gen'
	import { Button } from '$lib/components/common'
	import Select from '$lib/components/select/Select.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import DarkModeToggle from '$lib/components/sidebar/DarkModeToggle.svelte'
	import { readDevPreview, writeDevPreview, type DevAiState } from '$lib/devPreview'

	// Home as a given account would see it, in a frame beside the controls: the real page,
	// told by this browser's dev overrides (`$lib/devPreview`, `cloudHostedOverride`) what to
	// pretend. Every change reloads the frame, since those overrides are read at load.

	const CLOUD_KEY = 'cloudHostedOverride'
	const UNPINNED_KEY = 'home-ai-composer-collapsed'

	const AI_STATES: { value: DevAiState; label: string }[] = [
		{ value: 'real', label: "The workspace's real setup" },
		{ value: 'free-tier', label: 'Free Windmill AI, partly used' },
		{ value: 'free-tier-exhausted', label: 'Free Windmill AI, used up' },
		{ value: 'no-key', label: 'No AI key configured' },
		{ value: 'hidden', label: 'AI hidden in the workspace' }
	]
	const WIDTHS = [
		{ value: 'full', label: 'Full' },
		{ value: '1024', label: '1024' },
		{ value: '768', label: '768' },
		{ value: '400', label: '400' }
	]

	function readStorage(key: string): string | null {
		try {
			return localStorage.getItem(key)
		} catch {
			return null
		}
	}
	function writeStorage(key: string, value: string | undefined) {
		try {
			if (value === undefined) localStorage.removeItem(key)
			else localStorage.setItem(key, value)
		} catch {}
	}

	const initial = readDevPreview()
	let ai = $state<DevAiState>(initial?.ai ?? 'real')
	const USED_PCTS = [0, 30, 60, 90, 99]
	let usedPct = $state(initial?.freeTierUsedPct ?? 60)
	let cloud = $state(readStorage(CLOUD_KEY) === '1')
	let workspace = $state<string | undefined>(readStorage('workspace') ?? undefined)
	let width = $state('full')
	let workspaces = $state<{ label: string; value: string }[]>([])
	let reloadKey = $state(0)

	let src = $derived(workspace ? `/?workspace=${encodeURIComponent(workspace)}` : '/')

	function apply() {
		writeDevPreview({ ai, freeTierUsedPct: usedPct })
		writeStorage(CLOUD_KEY, cloud ? '1' : undefined)
		reloadKey++
	}

	function showComposerAgain() {
		writeStorage(UNPINNED_KEY, undefined)
		reloadKey++
	}

	onMount(async () => {
		const list = await WorkspaceService.listUserWorkspaces()
		workspaces = list.workspaces.map((w) => ({ label: `${w.name} (${w.id})`, value: w.id }))
	})
</script>

<svelte:head><title>Home preview</title></svelte:head>

<div class="flex h-screen bg-surface-secondary text-primary">
	<aside class="w-72 shrink-0 border-r bg-surface flex flex-col gap-5 p-4 overflow-y-auto">
		<div class="flex items-center justify-between">
			<h1 class="text-sm font-semibold text-emphasis">Home preview</h1>
			<DarkModeToggle />
		</div>

		{#if !import.meta.env.DEV}
			<p class="text-xs text-secondary">
				The overrides only work in a dev build (<code>npm run dev</code>).
			</p>
		{/if}

		<section class="flex flex-col gap-1.5">
			<span class="text-xs font-semibold text-emphasis">Workspace</span>
			<Select
				items={workspaces}
				bind:value={
					() => workspace,
					(v) => {
						workspace = v
						reloadKey++
					}
				}
				placeholder="Pick a workspace"
				size="sm"
			/>
			<span class="text-2xs text-hint">
				An empty one (e.g. <code>empty</code>) shows the first-visit state.
			</span>
		</section>

		<section class="flex flex-col gap-1.5">
			<span class="text-xs font-semibold text-emphasis">AI setup</span>
			<Select
				items={AI_STATES}
				bind:value={
					() => ai,
					(v) => {
						ai = v ?? 'real'
						apply()
					}
				}
				size="sm"
			/>
			{#if ai === 'free-tier'}
				<div class="flex flex-col gap-1 text-2xs text-secondary">
					<span>Used</span>
					<ToggleButtonGroup
						selected={String(usedPct)}
						onSelected={(v) => {
							usedPct = Number(v)
							apply()
						}}
					>
						{#snippet children({ item })}
							{#each USED_PCTS as p (p)}
								<ToggleButton value={String(p)} label="{p}%" {item} size="sm" />
							{/each}
						{/snippet}
					</ToggleButtonGroup>
				</div>
				<span class="text-2xs text-hint">
					Home hides the usage line; it shows in a session once it has a message.
				</span>
			{/if}
		</section>

		<section class="flex flex-col gap-1.5">
			<span class="text-xs font-semibold text-emphasis">Hosting</span>
			<Toggle
				size="sm"
				bind:checked={
					() => cloud,
					(v) => {
						cloud = v
						apply()
					}
				}
				options={{ right: 'Cloud (app.windmill.dev)' }}
			/>
			<span class="text-2xs text-hint">
				Cloud-only UI (quotas, upgrade) also needs a backend started with CLOUD_HOSTED.
			</span>
		</section>

		<section class="flex flex-col gap-1.5">
			<span class="text-xs font-semibold text-emphasis">Frame width</span>
			<ToggleButtonGroup selected={width} onSelected={(v) => (width = v)}>
				{#snippet children({ item })}
					{#each WIDTHS as w (w.value)}
						<ToggleButton value={w.value} label={w.label} {item} size="sm" />
					{/each}
				{/snippet}
			</ToggleButtonGroup>
		</section>

		<section class="flex flex-col gap-2">
			<Button
				unifiedSize="sm"
				variant="default"
				startIcon={{ icon: RotateCw }}
				onClick={() => reloadKey++}
			>
				Reload the frame
			</Button>
			<Button unifiedSize="sm" variant="subtle" onClick={showComposerAgain}>
				Show the composer again if unpinned
			</Button>
			<Button
				unifiedSize="sm"
				variant="subtle"
				startIcon={{ icon: SquareArrowOutUpRight }}
				href={src}
				target="_blank"
			>
				Open in a tab
			</Button>
		</section>

		<p class="text-2xs text-hint mt-auto">
			The settings are stored in this browser and apply to every tab of this dev server until set
			back to the real setup and cloud off.
		</p>
	</aside>

	<main class="flex-1 min-w-0 flex justify-center overflow-auto p-4">
		<div
			class="h-full border rounded-md overflow-hidden bg-surface shrink-0"
			style:width={width === 'full' ? '100%' : `${width}px`}
		>
			{#key reloadKey}
				<iframe title="Home" {src} class="w-full h-full"></iframe>
			{/key}
		</div>
	</main>
</div>
