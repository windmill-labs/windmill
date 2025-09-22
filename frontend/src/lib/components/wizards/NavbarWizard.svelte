<script lang="ts">
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { offset, flip, shift } from 'svelte-floating-ui/dom'
	import type { NavbarItem } from '../apps/editor/component'
	import Label from '../Label.svelte'
	import { getContext } from 'svelte'
	import Section from '../Section.svelte'
	import IconSelectInput from '../apps/editor/settingsPanel/inputEditor/IconSelectInput.svelte'
	import InputsSpecEditor from '../apps/editor/settingsPanel/InputsSpecEditor.svelte'
	import type { AppViewerContext } from '../apps/types'
	import Alert from '../common/alert/Alert.svelte'
	import OneOfInputSpecsEditor from '../apps/editor/settingsPanel/OneOfInputSpecsEditor.svelte'

	interface Props {
		value: NavbarItem
		trigger?: import('svelte').Snippet
	}

	let { value = $bindable(), trigger }: Props = $props()

	const { selectedComponent } = getContext<AppViewerContext>('AppViewerContext')

	const trigger_render = $derived(trigger)
</script>

<Popover
	floatingConfig={{
		strategy: 'fixed',
		placement: 'left-end',
		middleware: [offset(8), flip(), shift()]
	}}
	closeButton
	contentClasses="p-4 max-h-[70vh] overflow-y-auto"
>
	{#snippet trigger()}
		{@render trigger_render?.()}
	{/snippet}
	{#snippet content()}
		{#if value}
			<Section label="Navbar item" class="flex flex-col gap-2 w-80 overflow-y-auto max-h-screen">
				<InputsSpecEditor
					key={'Label'}
					bind:componentInput={value.label}
					id={$selectedComponent?.[0] ?? ''}
					userInputEnabled={false}
					shouldCapitalize={true}
					resourceOnly={false}
					fieldType={value.label?.['fieldType']}
					subFieldType={value.label?.['subFieldType']}
					format={value.label?.['format']}
					selectOptions={value.label?.['selectOptions']}
					fileUpload={value.label?.['fileUpload']}
					placeholder={value.label?.['placeholder']}
					customTitle={value.label?.['customTitle']}
					displayType={false}
				/>

				<OneOfInputSpecsEditor
					key={'Link'}
					bind:oneOf={value.path}
					id={$selectedComponent?.[0] ?? ''}
					shouldCapitalize={true}
					resourceOnly={false}
					inputSpecsConfiguration={value.path?.['configuration']}
					labels={value.path?.['labels']}
					tooltip={value.path?.['tooltip']}
				/>

				<Alert size="xs" title="Link Behavior" collapsible>
					<ul class="list-disc">
						<li>
							If you select an app, there are two cases:
							<div class="ml-2">
								<ul class="list-disc">
									<li>
										You selected the current app itself: Clicking on the link will highlight the
										item, and set the app in the output. Note that adding query params or an hash
										lets you distinguish between different items. Also note that query params can be
										retrieved from the context: `ctx.query`.
									</li>
									<li>
										You selected another app: Clicking on the link navigates to the selected app
										without reloading the page. In the editor, it will open in a new tab.
									</li>
								</ul>
							</div>
						</li>
						<li>
							If you select an external link, clicking on the link will navigate to the selected
							link in a new tab.
						</li>
					</ul>
				</Alert>

				<InputsSpecEditor
					key={'Disabled'}
					bind:componentInput={value.disabled}
					id={$selectedComponent?.[0] ?? ''}
					userInputEnabled={false}
					shouldCapitalize={true}
					resourceOnly={false}
					fieldType={value.disabled?.['fieldType']}
					subFieldType={value.disabled?.['subFieldType']}
					format={value.disabled?.['format']}
					selectOptions={value.disabled?.['selectOptions']}
					fileUpload={value.disabled?.['fileUpload']}
					placeholder={value.disabled?.['placeholder']}
					customTitle={value.disabled?.['customTitle']}
					displayType={false}
				/>
				<InputsSpecEditor
					key={'Hidden'}
					bind:componentInput={value.hidden}
					id={$selectedComponent?.[0] ?? ''}
					userInputEnabled={false}
					shouldCapitalize={true}
					resourceOnly={false}
					fieldType={value.hidden?.['fieldType']}
					subFieldType={value.hidden?.['subFieldType']}
					format={value.hidden?.['format']}
					selectOptions={value.hidden?.['selectOptions']}
					fileUpload={value.hidden?.['fileUpload']}
					placeholder={value.hidden?.['placeholder']}
					customTitle={value.hidden?.['customTitle']}
					displayType={false}
				/>

				<Label label="Icon" class="w-full">
					<IconSelectInput
						bind:value={value.icon}
						floatingConfig={{
							strategy: 'fixed',
							placement: 'left-end',
							middleware: [offset(8), flip(), shift()]
						}}
					/>
				</Label>
				<Label label="Caption">
					<input type="text" bind:value={value.caption} />
				</Label>
			</Section>
		{/if}
	{/snippet}
</Popover>
