<script lang="ts">
	import { Badge } from './common'
	import Toggle from './Toggle.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import { untrack } from 'svelte'
	import { ChevronRight } from 'lucide-svelte'

	interface Props {
		args?: Record<string, any>
	}

	let { args = $bindable({}) }: Props = $props()

	let verbose = $state(false)
	let debug = $state(false)
	let errorAction = $state(undefined as string | undefined)
	let collapsed = $state(true)
	let initialized = false

	const errorActionItems = [
		{ label: 'Stop', value: 'Stop' },
		{ label: 'Continue', value: 'Continue' },
		{ label: 'SilentlyContinue', value: 'SilentlyContinue' }
	]

	let activeBadges = $derived.by(() => {
		const badges: string[] = []
		if (verbose) badges.push('Verbose')
		if (debug) badges.push('Debug')
		if (errorAction) badges.push(`ErrorAction: ${errorAction}`)
		return badges
	})

	// Initialize toggles from pre-populated args (e.g. "Run again")
	$effect(() => {
		if (!initialized && args && Object.keys(args).length > 0) {
			initialized = true
			untrack(() => {
				verbose = args['_wm_ps_verbose'] === true
				debug = args['_wm_ps_debug'] === true
				errorAction = args['_wm_ps_error_action'] ?? undefined
			})
		}
	})

	// Sync toggles → args
	$effect(() => {
		const newArgs: Record<string, any> = {}
		if (verbose) newArgs['_wm_ps_verbose'] = true
		if (debug) newArgs['_wm_ps_debug'] = true
		if (errorAction) newArgs['_wm_ps_error_action'] = errorAction
		args = newArgs
	})
</script>

<div class="flex flex-col">
	<button
		class="flex items-center gap-1 text-xs text-secondary hover:text-primary transition-colors"
		onclick={() => (collapsed = !collapsed)}
	>
		<ChevronRight size={12} class="transition duration-200 {collapsed ? '' : 'rotate-90'}" />
		Common parameters
		{#if collapsed && activeBadges.length > 0}
			<div class="flex gap-1 ml-1">
				{#each activeBadges as label}
					<Badge color="blue">{label}</Badge>
				{/each}
			</div>
		{/if}
	</button>
	{#if !collapsed}
		<div class="flex flex-col gap-2 mt-2 ml-4">
			<Toggle options={{ right: 'Verbose' }} bind:checked={verbose} size="xs" />
			<Toggle options={{ right: 'Debug' }} bind:checked={debug} size="xs" />
			<div class="flex items-center gap-2">
				<span class="text-xs text-secondary">ErrorAction</span>
				<div class="w-40">
					<Select
						items={errorActionItems}
						bind:value={errorAction}
						placeholder="Default"
						clearable
						size="sm"
					/>
				</div>
			</div>
		</div>
	{/if}
</div>
