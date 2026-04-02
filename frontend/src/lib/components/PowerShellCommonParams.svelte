<script lang="ts">
	import { Section } from './common'
	import Toggle from './Toggle.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import { untrack } from 'svelte'

	interface Props {
		args?: Record<string, any>
	}

	let { args = $bindable({}) }: Props = $props()

	let verbose = $state(false)
	let debug = $state(false)
	let errorAction = $state(undefined as string | undefined)
	let initialized = false

	const errorActionItems = [
		{ label: 'Stop', value: 'Stop' },
		{ label: 'Continue', value: 'Continue' },
		{ label: 'SilentlyContinue', value: 'SilentlyContinue' }
	]

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

<Section label="PowerShell Common Parameters" collapsable initiallyCollapsed>
	<div class="flex flex-col gap-3">
		<Toggle options={{ right: '-Verbose' }} bind:checked={verbose} size="xs" />
		<Toggle options={{ right: '-Debug' }} bind:checked={debug} size="xs" />
		<div class="flex items-center gap-2">
			<span class="text-xs text-secondary">-ErrorAction</span>
			<div class="w-48">
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
</Section>
