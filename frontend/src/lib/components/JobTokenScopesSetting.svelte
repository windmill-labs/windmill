<script lang="ts">
	import Toggle from './Toggle.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ScopeSelector from './settings/ScopeSelector.svelte'

	interface Props {
		/** `null`/`undefined`: the job token has the full permissions of the identity it runs as. */
		value: string[] | null | undefined
		kind: 'script' | 'flow'
	}

	let { value = $bindable(), kind }: Props = $props()

	const OIDC_ONLY = ['oidc:write']

	type Mode = 'oidc_only' | 'no_api' | 'custom'

	function modeOf(scopes: string[]): Mode {
		if (scopes.length === 0) return 'no_api'
		if (scopes.length === 1 && scopes[0] === OIDC_ONLY[0]) return 'oidc_only'
		return 'custom'
	}

	// Kept apart from `value`: a custom list can hold exactly what a preset holds.
	let mode = $state<Mode>(modeOf(value ?? OIDC_ONLY))
	let customScopes = $state<string[]>(value && modeOf(value) === 'custom' ? [...value] : [])

	function setMode(next: Mode) {
		mode = next
		value = next === 'oidc_only' ? [...OIDC_ONLY] : next === 'no_api' ? [] : [...customScopes]
	}
</script>

<div class="flex flex-col gap-2">
	<Toggle
		size="sm"
		checked={value != null}
		on:change={() => {
			if (value != null) {
				value = null
			} else {
				setMode(mode)
			}
		}}
		options={{ right: 'Restrict the job token' }}
	/>
	{#if value != null}
		<ToggleButtonGroup selected={mode} onSelected={(v) => setMode(v)}>
			{#snippet children({ item })}
				<ToggleButton value="oidc_only" label="OIDC token only" {item} />
				<ToggleButton value="no_api" label="No API access" {item} />
				<ToggleButton value="custom" label="Custom" {item} />
			{/snippet}
		</ToggleButtonGroup>
		{#if mode === 'custom'}
			<ScopeSelector
				bind:selectedScopes={
					() => customScopes,
					(scopes) => {
						customScopes = scopes
						value = [...scopes]
					}
				}
				emptyLabel="No scopes selected. The job token can only reach its own job."
			/>
		{/if}
		<p class="text-2xs text-secondary">
			The <code>WM_TOKEN</code> of every job of this {kind} can only do what these scopes allow. Jobs
			it starts, {kind === 'flow' ? 'its steps, ' : ''}and AI agent tools inherit the restriction.
			Variables and resources passed as <code>$var:</code>/<code>$res:</code> inputs, an AI agent's provider
			resource, relative imports and object storage each need a read scope. Write scopes on scripts,
			flows, schedules or triggers let a job escape the restriction.
		</p>
	{/if}
</div>
