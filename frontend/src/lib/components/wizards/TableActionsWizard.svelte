<script lang="ts">
	import { Code } from 'lucide-svelte'
	import PanelSection from '../apps/editor/settingsPanel/common/PanelSection.svelte'
	import InputsSpecEditor from '../apps/editor/settingsPanel/InputsSpecEditor.svelte'
	import type { BaseAppComponent, RichConfiguration } from '../apps/types'
	import { Button } from '../common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'

	import { offset, flip, shift } from 'svelte-floating-ui/dom'
	import type {
		ButtonComponent,
		CheckboxComponent,
		SelectComponent
	} from '../apps/editor/component'
	export let actionsOrder: RichConfiguration | undefined = undefined
	export let selectedId: string | undefined = undefined
	export let components:
		| (BaseAppComponent & (ButtonComponent | CheckboxComponent | SelectComponent))[]
		| undefined
</script>

<Popover
	floatingConfig={{
		strategy: 'fixed',
		placement: 'left-end',
		middleware: [offset(8), flip(), shift()]
	}}
	closeButton
>
	<svelte:fragment slot="trigger">
		<slot name="trigger" />
	</svelte:fragment>
	<svelte:fragment slot="content">
		<div class="w-96">
			<PanelSection
				title={`Manage actions programmatically`}
				tooltip="
		You can manage the order of the actions programmatically: You need to return an array of action ids in the order you want them to appear in the table. You can also hide actions by not including them in the array."
			>
				<div class="w-full flex gap-2 flex-col mt-2">
					{#if actionsOrder}
						<InputsSpecEditor
							key={'Order'}
							bind:componentInput={actionsOrder}
							id={selectedId ?? ''}
							userInputEnabled={false}
							shouldCapitalize={true}
							resourceOnly={false}
							fieldType={actionsOrder?.['fieldType']}
							subFieldType={actionsOrder?.['subFieldType']}
							format={actionsOrder?.['format']}
							selectOptions={actionsOrder?.['selectOptions']}
							tooltip={actionsOrder?.['tooltip']}
							fileUpload={actionsOrder?.['fileUpload']}
							placeholder={actionsOrder?.['placeholder']}
							customTitle={actionsOrder?.['customTitle']}
							allowTypeChange={false}
							displayType={false}
						/>
						<Button
							size="xs"
							color="light"
							startIcon={{
								icon: Code
							}}
							variant="border"
							on:click={() => {
								actionsOrder = undefined
							}}
						>
							Disable
						</Button>
					{:else}
						<Button
							size="xs"
							color="light"
							startIcon={{
								icon: Code
							}}
							variant="border"
							on:click={() => {
								actionsOrder = {
									fieldType: 'text',
									type: 'evalv2',
									expr: JSON.stringify(components?.map((x) => x.id) ?? []),
									connections: []
								}
							}}
						>
							Enable
						</Button>
					{/if}
				</div>
			</PanelSection>
		</div>
	</svelte:fragment>
</Popover>
