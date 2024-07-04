<script lang="ts">
	import ConfirmationModal from './ConfirmationModal.svelte'
	import { beforeNavigate, goto } from '$app/navigation'
	import Button from '../button/Button.svelte'
	import type DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import { cleanValueProperties, orderedJsonStringify, type Value } from '$lib/utils'
	import { tick } from 'svelte'
	import { page } from '$app/stores'

	export let savedValue: Value | undefined = undefined
	export let modifiedValue: Value | undefined = undefined
	export let diffDrawer: DiffDrawer | undefined = undefined

	let bypassBeforeNavigate = false
	let open = false
	let goingTo: URL | undefined = undefined

	beforeNavigate(async (newNavigationState) => {
		// console.log('beforeNavigate', newNavigationState, bypassBeforeNavigate)
		if (
			!bypassBeforeNavigate &&
			newNavigationState.to &&
			newNavigationState.to.url != $page.url &&
			newNavigationState.to.url.pathname !== newNavigationState.from?.url.pathname
		) {
			// console.log('going to', newNavigationState.to.url)
			goingTo = newNavigationState.to.url
			newNavigationState.cancel()
			if (newNavigationState.type != 'popstate') {
				await tick() // make sure saved value is updated when clicking on save draft or deploy
			}
			if (savedValue && modifiedValue) {
				const draftOrDeployed = cleanValueProperties({
					...((savedValue.draft || savedValue) ?? {}),
					path: undefined
				})
				const current = cleanValueProperties({ ...(modifiedValue ?? {}), path: undefined })
				if (orderedJsonStringify(draftOrDeployed) === orderedJsonStringify(current)) {
					bypassBeforeNavigate = true
					console.log('FOO')
					goto(goingTo)
				} else {
					open = true
				}
			} else {
				open = true
			}
		} else if (bypassBeforeNavigate) {
			bypassBeforeNavigate = false
		}
	})
</script>

<ConfirmationModal
	{open}
	title="Unsaved changes detected"
	confirmationText="Discard changes"
	on:canceled={() => {
		open = false
	}}
	on:confirmed={() => {
		if (goingTo) {
			bypassBeforeNavigate = true
			goto(goingTo)
		}
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span>Are you sure you want to discard the changes you have made? </span>
		{#if savedValue && modifiedValue && diffDrawer}
			<Button
				wrapperClasses="self-start"
				color="light"
				variant="border"
				size="xs"
				on:click={() => {
					if (!savedValue || !modifiedValue) {
						return
					}
					open = false
					diffDrawer?.openDrawer()
					diffDrawer?.setDiff({
						mode: 'normal',
						deployed: savedValue,
						draft: savedValue.draft,
						current: modifiedValue,
						defaultDiffType: 'draft',
						button: {
							text: 'Leave anyway',
							onClick: () => {
								if (goingTo) {
									bypassBeforeNavigate = true
									goto(goingTo)
								}
							}
						}
					})
				}}
				>Show diff
			</Button>
		{/if}
	</div>
</ConfirmationModal>
