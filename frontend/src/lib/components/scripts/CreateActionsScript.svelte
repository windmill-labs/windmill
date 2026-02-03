<script lang="ts">
	import { Code2, Plus } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import { base } from '$lib/base'
	import { protectionRulesState, isDirectDeployBlocked, canBypassDirectDeployBlock } from '$lib/workspaceProtectionRules.svelte'
	import { userStore } from '$lib/stores'

	let { aiId, aiDescription }: { aiId: string; aiDescription: string } = $props()

	let rulesLoaded = $derived(protectionRulesState.rulesets !== undefined)
	let showCreateButton = $derived(rulesLoaded && (!isDirectDeployBlocked() || canBypassDirectDeployBlock($userStore)))
</script>

<!-- Buttons -->
{#if showCreateButton}
	<div class="flex flex-row gap-2">
		<Button
			id="create-script-button"
			{aiId}
			{aiDescription}
			unifiedSize="lg"
			variant="accent"
			startIcon={{ icon: Plus }}
			href="{base}/scripts/add"
			endIcon={{ icon: Code2 }}
		>
			Script
		</Button>
	</div>
{/if}
