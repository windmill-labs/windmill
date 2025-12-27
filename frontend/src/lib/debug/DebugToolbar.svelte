<script lang="ts">
	import { Bug, Play, Square, SkipForward, ArrowDownToLine, ArrowUpFromLine, Trash2 } from 'lucide-svelte'
	import { Button } from '$lib/components/common'

	interface Props {
		connected: boolean
		running: boolean
		stopped: boolean
		onStart: () => Promise<void>
		onStop: () => Promise<void>
		onContinue: () => Promise<void>
		onStepOver: () => Promise<void>
		onStepIn: () => Promise<void>
		onStepOut: () => Promise<void>
		onClearBreakpoints: () => void
	}

	let {
		connected,
		running,
		stopped,
		onStart,
		onStop,
		onContinue,
		onStepOver,
		onStepIn,
		onStepOut,
		onClearBreakpoints
	}: Props = $props()

	let loading = $state(false)

	async function handleStart(): Promise<void> {
		loading = true
		try {
			await onStart()
		} finally {
			loading = false
		}
	}

	async function handleStop(): Promise<void> {
		loading = true
		try {
			await onStop()
		} finally {
			loading = false
		}
	}
</script>

<div class="flex items-center gap-1 p-1 border-b border-surface-secondary bg-surface-secondary">
	<div class="flex items-center gap-1 mr-2">
		<Bug size={16} class="text-orange-500" />
		<span class="text-xs font-medium text-secondary">Debug</span>
		{#if connected}
			<span class="w-2 h-2 rounded-full bg-green-500" title="Connected"></span>
		{:else}
			<span class="w-2 h-2 rounded-full bg-gray-400" title="Disconnected"></span>
		{/if}
	</div>

	<div class="h-4 w-px bg-surface-tertiary mx-1"></div>

	{#if !connected || (!running && !stopped)}
		<Button
			size="xs"
			color="green"
			variant="contained"
			startIcon={{ icon: Play }}
			onclick={handleStart}
			disabled={loading}
		>
			Debug
		</Button>
	{:else}
		<Button
			size="xs"
			color="red"
			variant="contained"
			startIcon={{ icon: Square }}
			onclick={handleStop}
			disabled={loading}
		>
			Stop
		</Button>
	{/if}

	<div class="h-4 w-px bg-surface-tertiary mx-1"></div>

	<!-- Step controls - only enabled when stopped -->
	<div class="flex items-center gap-1">
		<Button
			size="xs"
			color="light"
			startIcon={{ icon: Play }}
			onclick={onContinue}
			disabled={!stopped}
			title="Continue (F5)"
			iconOnly
		/>
		<Button
			size="xs"
			color="light"
			startIcon={{ icon: SkipForward }}
			onclick={onStepOver}
			disabled={!stopped}
			title="Step Over (F10)"
			iconOnly
		/>
		<Button
			size="xs"
			color="light"
			startIcon={{ icon: ArrowDownToLine }}
			onclick={onStepIn}
			disabled={!stopped}
			title="Step Into (F11)"
			iconOnly
		/>
		<Button
			size="xs"
			color="light"
			startIcon={{ icon: ArrowUpFromLine }}
			onclick={onStepOut}
			disabled={!stopped}
			title="Step Out (Shift+F11)"
			iconOnly
		/>
	</div>

	<div class="h-4 w-px bg-surface-tertiary mx-1"></div>

	<Button
		size="xs"
		color="light"
		startIcon={{ icon: Trash2 }}
		onclick={onClearBreakpoints}
		title="Clear All Breakpoints"
		iconOnly
	/>

	{#if running && !stopped}
		<span class="ml-2 text-xs text-tertiary flex items-center gap-1">
			<span class="animate-pulse">Running...</span>
		</span>
	{:else if stopped}
		<span class="ml-2 text-xs text-orange-500 flex items-center gap-1">
			Paused
		</span>
	{/if}
</div>
