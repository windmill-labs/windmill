<script lang="ts">
	import AppConnectInner from '$lib/components/AppConnectInner.svelte'
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import { Button } from '$lib/components/common'
	import { workspaceStore } from '$lib/stores'
	import { onMount } from 'svelte'

	export let resourceType: string | undefined = undefined
	export let workspace: string
	export let express = false

	let step = 1
	let disabled = false
	let manual = true

	let appConnect: AppConnectInner | undefined = undefined

	let darkMode: boolean = false

	if (workspace) {
		$workspaceStore = workspace
	}

	onMount(async () => {
		if (resourceType) {
			appConnect?.open(resourceType)
		}
	})
</script>

<DarkModeObserver bind:darkMode />

<div>
	{#if !express}
		<div class="flex flex-row-reverse w-full pb-2">
			<div class="flex gap-2">
				{#if step > 2}
					<Button variant="border" on:click={appConnect?.back ?? (() => {})}>Back</Button>
				{/if}

				<Button {disabled} on:click={appConnect?.next ?? (() => {})}>
					{#if step == 2 && !manual}
						Connect
					{:else if step == 1}
						Next
					{:else}
						Save
					{/if}
				</Button>
			</div>
		</div>
	{/if}
	<AppConnectInner
		{express}
		bind:this={appConnect}
		bind:step
		bind:resourceType
		bind:disabled
		bind:manual
		on:error
		on:refresh
	/>
</div>
