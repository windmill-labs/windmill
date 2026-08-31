<script lang="ts">
	import AppConnectInner from '$lib/components/AppConnectInner.svelte'
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import { Button } from '$lib/components/common'
	import GoogleSigninButton from '$lib/components/GoogleSigninButton.svelte'
	import { workspaceStore } from '$lib/stores'
	import { onMount, untrack } from 'svelte'

	interface Props {
		resourceType?: string | undefined
		workspace: string
		express?: boolean
	}

	let { resourceType = $bindable(undefined), workspace, express = false }: Props = $props()

	let step = $state(1)
	let disabled = $state(false)
	let manual = $state(true)
	let isGoogleSignin = $state(false)

	let appConnect: AppConnectInner | undefined = $state(undefined)

	let darkMode: boolean = $state(false)

	if (untrack(() => workspace)) {
		$workspaceStore = untrack(() => workspace)
	}

	onMount(async () => {
		if (resourceType) {
			appConnect?.open(resourceType)
		}
	})
</script>

<DarkModeObserver bind:darkMode />

<!-- Column so the step-1 list, which fills its parent to scroll its rows on its own, has a
     height to fill here too. -->
<div class="flex flex-col h-full min-h-0">
	{#if !express}
		<div class="flex flex-row-reverse w-full pb-2 shrink-0">
			<div class="flex gap-2">
				{#if step > 2}
					<Button variant="default" unifiedSize="md" onClick={() => appConnect?.back()}>
						Back
					</Button>
				{/if}

				{#if isGoogleSignin}
					<GoogleSigninButton {disabled} onClick={() => appConnect?.next()} />
				{:else}
					<Button variant="accent" unifiedSize="md" {disabled} onClick={() => appConnect?.next()}>
						{#if step == 2 && !manual}
							Connect
						{:else if step == 1}
							Next
						{:else}
							Save
						{/if}
					</Button>
				{/if}
			</div>
		</div>
	{/if}
	<div class="flex-1 min-h-0">
		<AppConnectInner
			{express}
			bind:this={appConnect}
			bind:step
			bind:resourceType
			bind:disabled
			bind:manual
			bind:isGoogleSignin
			on:error
			on:refresh
		/>
	</div>
</div>
