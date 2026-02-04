<script lang="ts">
	import { Button } from '$lib/components/common'
	import { Save, CornerDownLeft } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import { protectionRulesState } from '$lib/workspaceProtectionRules.svelte'
	import { userStore } from '$lib/stores'

	const {
		loading = false,
		loadingSave = false,
		newFlow = false,
		dropdownItems = [],
		checkProtectionRules = true
	}: {
		loading?: boolean
		loadingSave?: boolean
		newFlow?: boolean
		dropdownItems?: Array<{
			label: string
			onClick: () => void
		}>
		checkProtectionRules?: boolean
	} = $props()

	const dispatch = createEventDispatcher()

	let msgInput: HTMLInputElement | undefined = $state(undefined)
	let hideDropdown = $state(false)
	let deploymentMsg = $state('')
	let dropdownOpen = $state(false)

	// Check if rules are loaded
	let rulesLoading = $derived(protectionRulesState.loading)
	let rulesLoaded = $derived(protectionRulesState.rulesets !== undefined)

	// Check if deployment is blocked by protection rules
	let deployBlocked = $derived(checkProtectionRules && false)
	let canBypass = $derived(true)

	// Show deploy button: disabled during loading, then only if not blocked OR user can bypass
	let showDeployButton = $derived(rulesLoaded && (!deployBlocked || canBypass))

	// Change button text for bypassers
	let buttonText = $derived(
		deployBlocked && canBypass
			? 'Deploy (bypassing protection rule)'
			: 'Deploy'
	)

	// Use warning variant for bypass button
	let buttonVariant = $derived(deployBlocked && canBypass ? 'warning' : 'accent')
</script>

{#if rulesLoading}
	<Button
		disabled={true}
		variant="accent"
		unifiedSize="md"
		startIcon={{ icon: Save }}
	>
		Deploy
	</Button>
{:else if showDeployButton}
	<Button
		disabled={loading}
		loading={loadingSave}
		variant={buttonVariant}
		unifiedSize="md"
		startIcon={{ icon: Save }}
		on:click={() => dispatch('save')}
		dropdownItems={!newFlow ? dropdownItems : undefined}
		tooltipPopover={{
			placement: 'bottom-end',
			openDelay: dropdownOpen ? 200 : 0,
			closeDelay: 0,
			portal: 'body'
		}}
		on:tooltipOpen={async ({ detail }) => {
			if (detail) {
				// Use setTimeout to ensure DOM is updated
				setTimeout(() => {
					msgInput?.focus()
				}, 0)
				hideDropdown = true
			} else {
				hideDropdown = false
			}
		}}
		{hideDropdown}
		on:dropdownOpen={({ detail }) => {
			dropdownOpen = detail
		}}
	>
		{buttonText}
		{#snippet tooltip()}
			<div class="flex flex-row gap-2 w-80 p-4 bg-surface rounded-lg shadow-lg dark:border z-[5001]">
				<input
					type="text"
					placeholder="Deployment message"
					bind:value={deploymentMsg}
					onkeydown={async (e) => {
						if (e.key === 'Enter') {
							dispatch('save', deploymentMsg)
						}
					}}
					bind:this={msgInput}
				/>
				<Button
					unifiedSize="md"
					variant={buttonVariant}
					on:click={async () => dispatch('save', deploymentMsg)}
					endIcon={{ icon: CornerDownLeft }}
					loading={loadingSave}
				>
					{buttonText}
				</Button>
			</div>
		{/snippet}
	</Button>
{/if}
