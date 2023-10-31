<script lang="ts">
	import ConfirmationModal from './ConfirmationModal.svelte'
	import { beforeNavigate, goto } from '$app/navigation'
	import { onDestroy } from 'svelte'
	import { dirtyStore } from './dirtyStore'
	import type { NewScript, NewScriptWithDraft, Script } from '$lib/gen'
	import Button from '../button/Button.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import { cleanScriptProperties } from '$lib/utils'
	import { deepEqual } from 'fast-equals'

	export let savedScript: NewScriptWithDraft | Script | undefined = undefined
	export let modifiedScript: NewScript | undefined = undefined

	let navigationState: { from: URL | undefined; to: URL | null; cancel: () => void } | undefined =
		undefined
	$: open = Boolean(navigationState)

	beforeNavigate((newNavigationState) => {
		if (
			!navigationState &&
			$dirtyStore &&
			newNavigationState.to &&
			newNavigationState.to.url.pathname !== newNavigationState.from?.url.pathname
		) {
			if (savedScript && modifiedScript) {
				const draftOrDeployed = cleanScriptProperties(savedScript['draft'] || savedScript)
				const current = cleanScriptProperties(modifiedScript)
				if (deepEqual(draftOrDeployed, current)) {
					return
				}
			}

			navigationState = {
				to: newNavigationState.to.url,
				from: newNavigationState.from?.url,
				cancel: newNavigationState.cancel
			}
			newNavigationState.cancel()
		}
	})

	onDestroy(() => {
		$dirtyStore = false
	})

	let diffDrawer: DiffDrawer
</script>

<DiffDrawer bind:this={diffDrawer} />

<ConfirmationModal
	{open}
	title="Unsaved changes detected"
	confirmationText="Discard changes"
	on:canceled={() => {
		if (navigationState) {
			navigationState.cancel()
		}
		navigationState = undefined
	}}
	on:confirmed={() => {
		if (navigationState?.to) {
			goto(navigationState.to)
		}
		$dirtyStore = false
		navigationState = undefined
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span>Are you sure you want to discard the changes you have made? </span>
		{#if savedScript && modifiedScript}
			<Button
				wrapperClasses="self-start"
				color="light"
				on:click={() => {
					if (!savedScript || !modifiedScript) {
						return
					}
					if (navigationState) {
						navigationState.cancel()
					}
					navigationState = undefined
					diffDrawer.openDrawer()
					const draftOfDeployed = cleanScriptProperties(savedScript['draft'] || savedScript)
					const current = cleanScriptProperties(modifiedScript)
					diffDrawer.setDiff(draftOfDeployed, current)
				}}
				>Show diff
			</Button>
		{/if}
	</div>
</ConfirmationModal>
