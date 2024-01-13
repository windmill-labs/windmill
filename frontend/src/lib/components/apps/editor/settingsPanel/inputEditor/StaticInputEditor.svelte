<script lang="ts">
	import type { InputType, StaticInput, StaticOptions } from '../../../inputType'
	import ArrayStaticInputEditor from '../ArrayStaticInputEditor.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import JsonEditor from './JsonEditor.svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '$lib/components/apps/types'
	import IconSelectInput from './IconSelectInput.svelte'
	import ColorInput from './ColorInput.svelte'
	import TabSelectInput from './TabSelectInput.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import SchemaEditor from '$lib/components/SchemaEditor.svelte'
	import autosize from 'svelte-autosize'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Settings } from 'lucide-svelte'
	import AgGridWizard from '$lib/components/wizards/AgGridWizard.svelte'
	import TableColumnWizard from '$lib/components/wizards/TableColumnWizard.svelte'
	import PlotlyWizard from '$lib/components/wizards/PlotlyWizard.svelte'
	import ChartJSWizard from '$lib/components/wizards/ChartJSWizard.svelte'
	import AgChartWizard from '$lib/components/wizards/AgChartWizard.svelte'
	import DBExplorerWizard from '$lib/components/wizards/DBExplorerWizard.svelte'
	import Label from '$lib/components/Label.svelte'

	export let componentInput: StaticInput<any> | undefined
	export let fieldType: InputType | undefined = undefined
	export let subFieldType: InputType | undefined = undefined
	export let selectOptions: StaticOptions['selectOptions'] | undefined = undefined
	export let placeholder: string | undefined = undefined
	export let format: string | undefined = undefined

	const { onchange } = getContext<AppViewerContext>('AppViewerContext')

	$: componentInput && onchange?.()
</script>

{#key subFieldType}
	{#if componentInput?.type === 'static'}
		{#if fieldType === 'number' || fieldType === 'integer'}
			<input on:keydown|stopPropagation type="number" bind:value={componentInput.value} />
		{:else if fieldType === 'textarea'}
			<textarea use:autosize on:keydown|stopPropagation bind:value={componentInput.value} />
		{:else if fieldType === 'date'}
			<input on:keydown|stopPropagation type="date" bind:value={componentInput.value} />
		{:else if fieldType === 'boolean'}
			<Toggle bind:checked={componentInput.value} size="xs" />
		{:else if fieldType === 'select' && selectOptions}
			<select on:keydown|stopPropagation bind:value={componentInput.value}>
				{#each selectOptions ?? [] as option}
					{#if typeof option == 'string'}
						<option value={option}>
							{option}
						</option>
					{:else}
						<option value={option.value}>
							{option.label}
						</option>
					{/if}
				{/each}
			</select>
		{:else if fieldType === 'icon-select'}
			<IconSelectInput bind:componentInput />
		{:else if fieldType === 'tab-select'}
			<TabSelectInput bind:componentInput />
		{:else if fieldType === 'resource' && subFieldType !== 's3'}
			<ResourcePicker
				initialValue={componentInput.value?.split('$res:')?.[1] || ''}
				on:change={(e) => {
					let path = e.detail
					if (componentInput) {
						if (path) {
							componentInput.value = `$res:${path}`
						} else {
							componentInput.value = undefined
						}
					}
				}}
				showSchemaExplorer
				resourceType="postgresql"
			/>
		{:else if fieldType === 'resource' && subFieldType === 's3'}
			<ResourcePicker
				initialValue={componentInput.value?.split('$res:')?.[1] || ''}
				on:change={(e) => {
					let path = e.detail
					if (componentInput) {
						if (path) {
							componentInput.value = `$res:${path}`
						} else {
							componentInput.value = undefined
						}
					}
				}}
				resourceType="s3"
			/>
		{:else if fieldType === 'labeledresource'}
			{#if componentInput?.value && typeof componentInput?.value == 'object' && 'label' in componentInput?.value && (componentInput.value?.['value'] == undefined || typeof componentInput.value?.['value'] == 'string')}
				<div class="flex flex-col gap-1 w-full">
					<input
						on:keydown|stopPropagation
						placeholder="Label"
						type="text"
						bind:value={componentInput.value['label']}
					/>
					<ResourcePicker
						initialValue={componentInput.value?.['value']?.split('$res:')?.[1] || ''}
						on:change={(e) => {
							let path = e.detail
							if (componentInput) {
								if (path) {
									componentInput.value['value'] = `$res:${path}`
								} else {
									componentInput.value['value'] = undefined
								}
							}
						}}
						showSchemaExplorer
					/>
				</div>
			{:else}
				Inconsistent labeled resource object
			{/if}
		{:else if fieldType === 'color'}
			<ColorInput bind:value={componentInput.value} />
		{:else if fieldType === 'object' || fieldType == 'labeledselect'}
			{#if format?.startsWith('resource-') && (componentInput.value == undefined || typeof componentInput.value == 'string')}
				<ResourcePicker
					initialValue={componentInput.value?.split('$res:')?.[1] || ''}
					on:change={(e) => {
						let path = e.detail
						if (componentInput) {
							if (path) {
								componentInput.value = `$res:${path}`
							} else {
								componentInput.value = undefined
							}
						}
					}}
					resourceType={format && format?.split('-').length > 1
						? format.substring('resource-'.length)
						: undefined}
					showSchemaExplorer
				/>
			{:else}
				<div class="flex w-full flex-col">
					<JsonEditor
						small
						bind:value={componentInput.value}
						code={JSON.stringify(componentInput.value, null, 2)}
					/>
				</div>
			{/if}
		{:else if fieldType === 'array'}
			<ArrayStaticInputEditor {subFieldType} bind:componentInput on:deleteArrayItem />
		{:else if fieldType === 'schema'}
			<div class="w-full">
				<SchemaEditor bind:schema={componentInput.value} lightMode />
			</div>
		{:else if fieldType === 'ag-grid'}
			<div class="flex flex-row rounded-md bg-surface items-center h-full">
				<div class="relative w-full">
					<input
						class="text-xs px-2 border-y w-full flex flex-row items-center border-r rounded-r-md h-8"
						bind:value={componentInput.value.field}
						placeholder="Field"
					/>
					<div class="absolute top-1 right-1">
						<AgGridWizard bind:value={componentInput.value}>
							<svelte:fragment slot="trigger">
								<Button color="light" size="xs2" nonCaptureEvent={true}>
									<div class="flex flex-row items-center gap-2 text-xs font-normal">
										<Settings size={16} />
									</div>
								</Button>
							</svelte:fragment>
						</AgGridWizard>
					</div>
				</div>
			</div>
		{:else if fieldType === 'db-explorer' && componentInput.value != undefined}
			<div class="flex flex-row rounded-md bg-surface items-center h-full">
				<div class="relative w-full">
					<input
						class="text-xs px-2 border-y w-full flex flex-row items-center border-r rounded-r-md h-8"
						bind:value={componentInput.value.field}
						placeholder="Field"
						disabled
					/>
					<div class="absolute top-1 right-1">
						<DBExplorerWizard bind:value={componentInput.value}>
							<svelte:fragment slot="trigger">
								<Button color="light" size="xs2" nonCaptureEvent={true}>
									<div class="flex flex-row items-center gap-2 text-xs font-normal">
										<Settings size={16} />
									</div>
								</Button>
							</svelte:fragment>
						</DBExplorerWizard>
					</div>
				</div>
			</div>
		{:else if fieldType === 'table-column'}
			<div class="flex flex-row rounded-md bg-surface items-center h-full">
				<div class="relative w-full">
					<input
						class="text-xs px-2 border-y w-full flex flex-row items-center border-r rounded-r-md h-8"
						bind:value={componentInput.value.field}
						placeholder="Field"
					/>
					<div class="absolute top-1 right-1">
						<TableColumnWizard bind:column={componentInput.value}>
							<svelte:fragment slot="trigger">
								<Button color="light" size="xs2" nonCaptureEvent={true}>
									<div class="flex flex-row items-center gap-2 text-xs font-normal">
										<Settings size={16} />
									</div>
								</Button>
							</svelte:fragment>
						</TableColumnWizard>
					</div>
				</div>
			</div>
		{:else if fieldType === 'plotly'}
			<div class="flex flex-row rounded-md bg-surface items-center h-full">
				<div class="relative w-full">
					<input
						class="text-xs px-2 border-y w-full flex flex-row items-center border-r rounded-r-md h-8"
						bind:value={componentInput.value.name}
						placeholder="Dataset name"
					/>
					<div class="absolute top-1 right-1">
						<PlotlyWizard bind:value={componentInput.value} on:remove>
							<svelte:fragment slot="trigger">
								<Button color="light" size="xs2" nonCaptureEvent={true}>
									<div class="flex flex-row items-center gap-2 text-xs font-normal">
										<Settings size={16} />
									</div>
								</Button>
							</svelte:fragment>
						</PlotlyWizard>
					</div>
				</div>
			</div>
		{:else if fieldType === 'chartjs'}
			<div class="flex flex-row rounded-md bg-surface items-center h-full">
				<div class="relative w-full">
					<input
						class="text-xs px-2 border-y w-full flex flex-row items-center border-r rounded-r-md h-8"
						bind:value={componentInput.value.name}
						placeholder="Dataset name"
					/>
					<div class="absolute top-1 right-1">
						<ChartJSWizard bind:value={componentInput.value} on:remove>
							<svelte:fragment slot="trigger">
								<Button color="light" size="xs2" nonCaptureEvent={true}>
									<div class="flex flex-row items-center gap-2 text-xs font-normal">
										<Settings size={16} />
									</div>
								</Button>
							</svelte:fragment>
						</ChartJSWizard>
					</div>
				</div>
			</div>
		{:else if fieldType === 'ag-chart'}
			<div class="flex flex-row rounded-md bg-surface items-center h-full">
				<div class="relative w-full">
					<input
						class="text-xs px-2 border-y w-full flex flex-row items-center border-r rounded-r-md h-8"
						bind:value={componentInput.value.name}
						placeholder="Dataset name"
					/>

					<div class="absolute top-1 right-1">
						<AgChartWizard bind:value={componentInput.value} on:remove>
							<svelte:fragment slot="trigger">
								<Button color="light" size="xs2" nonCaptureEvent={true}>
									<div class="flex flex-row items-center gap-2 text-xs font-normal">
										<Settings size={16} />
									</div>
								</Button>
							</svelte:fragment>
						</AgChartWizard>
					</div>
				</div>
			</div>
		{:else if fieldType === 'number-tuple'}
			<div class="flex flex-row rounded-md bg-surface items-center h-full">
				<div class="relative w-full flex flex-row gap-2">
					<Label label="Y Low">
						<input
							class="text-xs px-2 border-y w-full flex flex-row items-center border-r rounded-r-md h-8"
							bind:value={componentInput.value[0]}
							type="number"
						/>
					</Label>
					<Label label="Y High">
						<input
							class="text-xs px-2 border-y w-full flex flex-row items-center border-r rounded-r-md h-8"
							bind:value={componentInput.value[1]}
							type="number"
						/>
					</Label>
				</div>
			</div>
		{:else}
			<div class="flex gap-1 relative w-full">
				<textarea
					rows="1"
					use:autosize
					on:keydown|stopPropagation
					placeholder={placeholder ?? 'Static value'}
					bind:value={componentInput.value}
					class="!pr-12"
				/>
			</div>
		{/if}
	{/if}
{/key}
