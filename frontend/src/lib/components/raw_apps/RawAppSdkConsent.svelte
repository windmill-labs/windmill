<script lang="ts">
	import { Button } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import { ShieldCheck } from 'lucide-svelte'
	import { sdkScopeDescription, sdkScopeLabel } from './sdkScopes'

	let {
		scopes,
		onContinue,
		onDecline
	}: {
		/** Scopes the app policy declares for its frontend SDK token. */
		scopes: string[]
		/** Fired when the viewer accepts; `dontAskAgain` persists the consent for
		 * this app path so the prompt is skipped until the declared scopes grow. */
		onContinue: (dontAskAgain: boolean) => void
		/** Fired when the viewer declines: the app still renders, its frontend code
		 * just gets no token (SDK calls fail). */
		onDecline: () => void
	} = $props()

	let dontAskAgain = $state(false)
</script>

<div class="px-4 mt-20 max-w-xl mx-auto">
	<div class="border rounded-md p-6 bg-surface shadow-sm flex flex-col gap-4">
		<div class="flex items-center gap-2">
			<ShieldCheck size={20} class="text-primary" />
			<div class="text-lg font-semibold">This app requires the following permissions</div>
		</div>
		<p class="text-sm text-secondary">
			The app's code will be able to call the Windmill API on your behalf, restricted to:
		</p>
		<ul class="flex flex-col gap-2">
			{#each scopes as scope (scope)}
				<li class="text-sm">
					<span class="font-medium">{sdkScopeLabel(scope)}</span>
					{#if sdkScopeDescription(scope)}
						<span class="text-tertiary"> — {sdkScopeDescription(scope)}</span>
					{/if}
				</li>
			{/each}
		</ul>
		<div class="flex items-center justify-between gap-4 pt-2">
			<Toggle bind:checked={dontAskAgain} size="xs" options={{ right: 'Do not ask again' }} />
			<div class="flex items-center gap-2">
				<Button variant="default" onclick={onDecline}>Open without granting</Button>
				<Button variant="accent" onclick={() => onContinue(dontAskAgain)}>Continue</Button>
			</div>
		</div>
	</div>
</div>
