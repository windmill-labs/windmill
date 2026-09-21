<script lang="ts">
	import ConfirmationModal from './ConfirmationModal.svelte'
	import Button from '../button/Button.svelte'
	import { type Value } from '$lib/utils'
	import type { DiffDrawerI } from '$lib/components/diff_drawer'

	interface Props {
		deployedValue?: Value | undefined
		currentValue?: Value | undefined
		diffDrawer?: DiffDrawerI | undefined
		confirmCallback: () => void
		deployedBy?: string | undefined
		open?: boolean
		/** Takes a drawer opening on the host editor's behalf, so the diff below is one the
		 *  editor can take back down. Override anyway deploys that editor's content: without
		 *  this the drawer would outlive it (a relocation remounts the editor) and deploy it
		 *  at a path it has left. Omit where the host cannot be remounted under the drawer. */
		claimOpening?: () => number | undefined
		/** The host could not read the deployed head, so this confirmation is caution rather
		 *  than an observed newer version: nobody may have deployed over the user, and
		 *  `deployedBy` is then whoever wrote the head, possibly themselves. */
		headUnknown?: boolean
	}

	let {
		deployedValue = $bindable(),
		currentValue = undefined,
		diffDrawer = undefined,
		confirmCallback,
		deployedBy = undefined,
		open = $bindable(false),
		claimOpening = undefined,
		headUnknown = false
	}: Props = $props()
</script>

<ConfirmationModal
	{open}
	title={headUnknown ? 'Deploy anyway?' : 'New version deployed by ' + deployedBy}
	confirmationText="Override"
	on:canceled={() => {
		open = false
	}}
	on:confirmed={() => confirmCallback()}
>
	<div class="flex flex-col w-full space-y-4">
		<span>
			{headUnknown
				? 'This editor could not check whether a newer version is deployed, so it cannot tell whether this overwrites newer work.'
				: 'A new version was deployed while you were editing this one.'}
		</span>
		{#if diffDrawer}
			<Button
				wrapperClasses="self-start"
				variant="default"
				size="xs"
				on:click={() => {
					if (!deployedValue || !currentValue) {
						return
					}
					open = false
					const opening = claimOpening?.()
					diffDrawer?.openDrawer(opening)
					diffDrawer?.setDiff(
						{
							mode: 'simple',
							original: deployedValue,
							current: currentValue,
							title: 'Deployed <> Current',
							button: {
								text: 'Override anyway',
								onClick: () => confirmCallback()
							}
						},
						opening
					)
				}}
				>Show diff
			</Button>
		{/if}
	</div>
</ConfirmationModal>
