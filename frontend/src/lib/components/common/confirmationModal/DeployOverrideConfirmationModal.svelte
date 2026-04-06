<script lang="ts">
	import ConfirmationModal from './ConfirmationModal.svelte'
	import Button from '../button/Button.svelte'
	import { t } from '$lib/i18n/t.svelte'
	import { type Value } from '$lib/utils'
	import type { DiffDrawerI } from '$lib/components/diff_drawer'

	interface Props {
		deployedValue?: Value | undefined
		currentValue?: Value | undefined
		diffDrawer?: DiffDrawerI | undefined
		confirmCallback: () => void
		deployedBy?: string | undefined
		open?: boolean
	}

	let {
		deployedValue = $bindable(),
		currentValue = undefined,
		diffDrawer = undefined,
		confirmCallback,
		deployedBy = undefined,
		open = $bindable(false)
	}: Props = $props()
</script>

<ConfirmationModal
	{open}
	title={'New version deployed by ' + deployedBy}
	confirmationText={t('confirm.override')}
	on:canceled={() => {
		open = false
	}}
	on:confirmed={() => confirmCallback()}
>
	<div class="flex flex-col w-full space-y-4">
		<span>{t('confirm.new_version_deployed')}</span>
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
					diffDrawer?.openDrawer()
					diffDrawer?.setDiff({
						mode: 'simple',
						original: deployedValue,
						current: currentValue,
						title: 'Deployed <> Current',
						button: {
							text: t('confirm.override_anyway'),
							onClick: () => confirmCallback()
						}
					})
				}}
				>{t('editor.show_diff')}
			</Button>
		{/if}
	</div>
</ConfirmationModal>
