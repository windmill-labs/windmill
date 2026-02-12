<script lang="ts">
	import { createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import type { InputType, StaticInput, StaticOptions } from '../../../inputType'
	import ArrayStaticInputEditor from '../ArrayStaticInputEditor.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '$lib/components/apps/types'
	import IconSelectInput from './IconSelectInput.svelte'
	import ColorInput from './ColorInput.svelte'
	import TabSelectInput from './TabSelectInput.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import autosize from '$lib/autosize'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Loader2, Pipette, Settings } from 'lucide-svelte'
	import AgGridWizard from '$lib/components/wizards/AgGridWizard.svelte'
	import TableColumnWizard from '$lib/components/wizards/TableColumnWizard.svelte'
	import PlotlyWizard from '$lib/components/wizards/PlotlyWizard.svelte'
	import ChartJSWizard from '$lib/components/wizards/ChartJSWizard.svelte'
	import AgChartWizard from '$lib/components/wizards/AgChartWizard.svelte'
	import DBExplorerWizard from '$lib/components/wizards/DBExplorerWizard.svelte'
	import Label from '$lib/components/Label.svelte'
	import DateTimeInput from '$lib/components/DateTimeInput.svelte'
	import DBTableSelect from './DBTableSelect.svelte'
	import EditableSchemaDrawer from '$lib/components/schema/EditableSchemaDrawer.svelte'
	import AppPicker from '$lib/components/wizards/AppPicker.svelte'
	import JsonEditor from '$lib/components/JsonEditor.svelte'
	import S3FilePicker from '$lib/components/S3FilePicker.svelte'
	import FileUpload from '$lib/components/common/fileUpload/FileUpload.svelte'
	import DucklakePicker from '$lib/components/DucklakePicker.svelte'
	import type SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import DatatablePicker from '$lib/components/DatatablePicker.svelte'

	interface Props {
		componentInput: StaticInput<any> | undefined
		fieldType?: InputType | undefined
		subFieldType?: InputType | undefined
		selectOptions?: StaticOptions['selectOptions'] | undefined
		placeholder?: string | undefined
		format?: string | undefined
		id: string | undefined
	}

	let {
		componentInput = $bindable(),
		fieldType = undefined,
		subFieldType = undefined,
		selectOptions = undefined,
		placeholder = undefined,
		format = undefined,
		id
	}: Props = $props()

	const appContext = getContext<AppViewerContext>('AppViewerContext')

	$effect(() => {
		componentInput && appContext?.onchange?.()
	})

	let s3FileUploadRawMode = $state(
		componentInput?.value && typeof componentInput.value == 'object' && !!componentInput.value?.s3
	)
	let s3FilePicker: S3FilePicker | undefined = $state(undefined)
	let s3JsonEditor: SimpleEditor | undefined = $state()
</script>

{#key subFieldType}
	{#if componentInput?.type === 'static'}
		{#if fieldType === 'number' || fieldType === 'integer'}
			<input
				onkeydown={stopPropagation(bubble('keydown'))}
				type="number"
				bind:value={componentInput.value}
			/>
		{:else if fieldType === 'textarea'}
			<textarea
				use:autosize
				onkeydown={stopPropagation(bubble('keydown'))}
				bind:value={componentInput.value}
			></textarea>
		{:else if fieldType === 'date'}
			<input
				onkeydown={stopPropagation(bubble('keydown'))}
				type="date"
				bind:value={componentInput.value}
			/>
		{:else if fieldType === 'time'}
			<input
				onkeydown={stopPropagation(bubble('keydown'))}
				type="time"
				bind:value={componentInput.value}
			/>
		{:else if fieldType === 'datetime'}
			<DateTimeInput bind:value={componentInput.value} />
		{:else if fieldType === 'boolean'}
			<Toggle bind:checked={componentInput.value} size="xs" class="mt-2" />
		{:else if fieldType === 'select' && selectOptions}
			{#if subFieldType === 'db-table'}
				<DBTableSelect bind:componentInput {selectOptions} {id} />
			{:else}
				<select onkeydown={stopPropagation(bubble('keydown'))} bind:value={componentInput.value}>
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
			{/if}
		{:else if fieldType === 'icon-select'}
			<IconSelectInput bind:value={componentInput.value} />
		{:else if fieldType === 'tab-select'}
			<TabSelectInput bind:componentInput />
		{:else if fieldType === 'resource' && subFieldType && ['mysql', 'postgres', 'ms_sql_server', 'snowflake', 'snowflake_oauth', 'bigquery', 'oracledb'].includes(subFieldType)}
			<ResourcePicker
				bind:value={
					() => (componentInput?.value ? componentInput?.value?.split('$res:')?.[1] : undefined),
					(v) => {
						if (componentInput) {
							if (v) {
								componentInput.value = `$res:${v}`
							} else {
								componentInput.value = undefined
							}
						}
					}
				}
				showSchemaExplorer
				resourceType={subFieldType === 'postgres' ? 'postgresql' : subFieldType}
			/>
		{:else if fieldType === 'resource' && subFieldType === 's3'}
			<ResourcePicker
				placeholder="S3 resource (workspace s3 if empty)"
				bind:value={
					() => (componentInput?.value ? componentInput?.value?.split('$res:')?.[1] : undefined),
					(v) => {
						if (componentInput) {
							if (v) {
								componentInput.value = `$res:${v}`
							} else {
								componentInput.value = undefined
							}
						}
					}
				}
				resourceType="s3"
			/>
		{:else if fieldType === 'ducklake'}
			<DucklakePicker
				class="w-full"
				bind:value={
					() => componentInput?.value?.split('ducklake://')?.[1],
					(v) => componentInput && (componentInput.value = v ? `ducklake://${v}` : undefined)
				}
				showSchemaExplorer
			/>
		{:else if fieldType === 'datatable'}
			<DatatablePicker
				class="w-full"
				bind:value={
					() => componentInput?.value?.split('datatable://')?.[1],
					(v) => componentInput && (componentInput.value = v ? `datatable://${v}` : undefined)
				}
				showSchemaExplorer
			/>
		{:else if fieldType === 'labeledresource'}
			{#if componentInput?.value && typeof componentInput?.value == 'object' && 'label' in componentInput?.value && (componentInput.value?.['value'] == undefined || typeof componentInput.value?.['value'] == 'string')}
				<div class="flex flex-col gap-1 w-full">
					<input
						onkeydown={stopPropagation(bubble('keydown'))}
						placeholder="Label"
						type="text"
						bind:value={
							() => componentInput?.value?.['label'],
							(v) => {
								if (componentInput) {
									componentInput.value['label'] = v
								}
								componentInput = $state.snapshot(componentInput)
							}
						}
					/>
					<ResourcePicker
						bind:value={
							() =>
								componentInput?.value
									? componentInput?.value?.['value']?.split('$res:')?.[1]
									: undefined,
							(v) => {
								if (componentInput) {
									if (v) {
										componentInput.value['value'] = `$res:${v}`
									} else {
										componentInput.value['value'] = undefined
									}
									componentInput = $state.snapshot(componentInput)
								}
							}
						}
						showSchemaExplorer
					/>
				</div>
			{:else}
				Inconsistent labeled resource object
			{/if}
		{:else if fieldType === 'color'}
			<ColorInput bind:value={componentInput.value} />
		{:else if fieldType === 'object' || fieldType == 'labeledselect'}
			{#if format && format.split('-').length > 1 && format
					.replace('resource-', '')
					.replace('_', '')
					.toLowerCase() == 's3object'}
				<div class="flex flex-col w-full gap-1">
					<Toggle
						class="flex justify-end"
						bind:checked={s3FileUploadRawMode}
						size="xs"
						options={{ left: 'Raw S3 object input' }}
					/>
					{#if s3FileUploadRawMode}
						{#await import('$lib/components/JsonEditor.svelte')}
							<Loader2 class="animate-spin" />
						{:then Module}
							<Module.default
								code={JSON.stringify(componentInput.value ?? { s3: '' }, null, 2)}
								bind:value={componentInput.value}
								bind:editor={s3JsonEditor}
							/>
						{/await}
					{:else}
						<FileUpload
							allowMultiple={false}
							randomFileKey={true}
							on:addition={(evt) => {
								if (componentInput) {
									componentInput.value = {
										s3: evt.detail?.path ?? '',
										filename: evt.detail?.filename ?? ''
									}
									s3FileUploadRawMode = true
								}
							}}
							on:deletion={(evt) => {
								if (componentInput) {
									componentInput.value = {
										s3: ''
									}
								}
							}}
						/>
					{/if}
					<Button
						variant="default"
						size="xs"
						btnClasses="mt-1"
						on:click={() => {
							s3FilePicker?.open?.()
						}}
						startIcon={{ icon: Pipette }}
					>
						Choose an object from the catalog
					</Button>
				</div>
				<S3FilePicker
					bind:this={s3FilePicker}
					readOnlyMode={false}
					onSelectAndClose={(selected) => {
						if (componentInput) {
							componentInput.value = selected
							s3JsonEditor?.setCode(JSON.stringify(selected, null, 2))
						}
						s3FileUploadRawMode = true
					}}
				/>
			{:else if format?.startsWith('resource-') && (componentInput.value == undefined || typeof componentInput.value == 'string')}
				<ResourcePicker
					bind:value={
						() => (componentInput?.value ? componentInput?.value?.split('$res:')?.[1] : undefined),
						(v) => {
							if (componentInput) {
								if (v) {
									componentInput.value = `$res:${v}`
								} else {
									componentInput.value = undefined
								}
							}
						}
					}
					resourceType={format && format?.split('-').length > 1
						? format.substring('resource-'.length)
						: undefined}
					showSchemaExplorer
				/>
			{:else if fieldType == 'labeledselect' && typeof componentInput.value == 'string'}
				<input
					class="text-xs px-2 border-y w-full flex flex-row items-center border-r rounded-r-md h-8"
					bind:value={componentInput.value}
					placeholder="Label"
				/>
			{:else}
				<div class="flex w-full flex-col">
					<JsonEditor
						loadAsync
						small
						bind:value={componentInput.value}
						code={JSON.stringify(componentInput.value, null, 2)}
					/>
				</div>
			{/if}
		{:else if fieldType === 'array'}
			<ArrayStaticInputEditor {id} {subFieldType} bind:componentInput on:deleteArrayItem />
		{:else if fieldType === 'schema'}
			<div class="w-full">
				<EditableSchemaDrawer bind:schema={componentInput.value} />
			</div>
		{:else if fieldType === 'ag-grid'}
			<div class="flex flex-row rounded-md bg-surface items-center h-full">
				<div class="relative w-full">
					{#if componentInput.value._isActionsColumn === true}
						<div
							class="text-xs px-2 border w-full flex flex-row items-center rounded-r-md h-8 text-primary"
						>
							Actions Column
						</div>
					{:else}
						<input
							class="text-xs px-2 border-y w-full flex flex-row items-center border-r rounded-r-md h-8"
							bind:value={componentInput.value.field}
							placeholder="Field"
						/>
					{/if}
					<div class="absolute top-1 right-1">
						<AgGridWizard bind:value={componentInput.value}>
							{#snippet trigger()}
								<Button color="light" size="xs2" nonCaptureEvent={true}>
									<div class="flex flex-row items-center gap-2 text-xs font-normal">
										<Settings size={16} />
									</div>
								</Button>
							{/snippet}
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
							{#snippet trigger()}
								<Button color="light" size="xs2" nonCaptureEvent={true}>
									<div class="flex flex-row items-center gap-2 text-xs font-normal">
										<Settings size={16} />
									</div>
								</Button>
							{/snippet}
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
							{#snippet trigger()}
								<Button color="light" size="xs2" nonCaptureEvent={true}>
									<div class="flex flex-row items-center gap-2 text-xs font-normal">
										<Settings size={16} />
									</div>
								</Button>
							{/snippet}
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
							{#snippet trigger()}
								<Button color="light" size="xs2" nonCaptureEvent={true}>
									<div class="flex flex-row items-center gap-2 text-xs font-normal">
										<Settings size={16} />
									</div>
								</Button>
							{/snippet}
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
							{#snippet trigger()}
								<Button color="light" size="xs2" nonCaptureEvent={true}>
									<div class="flex flex-row items-center gap-2 text-xs font-normal">
										<Settings size={16} />
									</div>
								</Button>
							{/snippet}
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
							{#snippet trigger()}
								<Button color="light" size="xs2" nonCaptureEvent={true}>
									<div class="flex flex-row items-center gap-2 text-xs font-normal">
										<Settings size={16} />
									</div>
								</Button>
							{/snippet}
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
		{:else if fieldType === 'app-path'}
			<AppPicker
				bind:value={
					() => componentInput!.value,
					(v) => {
						componentInput!.value = v
						componentInput = $state.snapshot(componentInput)
					}
				}
			/>
		{:else}
			<div class="flex gap-1 relative w-full">
				<textarea
					rows="1"
					use:autosize
					onkeydown={stopPropagation(bubble('keydown'))}
					placeholder={placeholder ?? 'Static value'}
					bind:value={componentInput.value}
					class="!pr-12"
				></textarea>
			</div>
		{/if}
	{/if}
{/key}
