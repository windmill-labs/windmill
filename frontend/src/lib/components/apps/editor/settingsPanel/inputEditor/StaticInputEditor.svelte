<script lang="ts">
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

	export let componentInput: StaticInput<any> | undefined
	export let fieldType: InputType | undefined = undefined
	export let subFieldType: InputType | undefined = undefined
	export let selectOptions: StaticOptions['selectOptions'] | undefined = undefined
	export let placeholder: string | undefined = undefined
	export let format: string | undefined = undefined
	export let id: string | undefined

	const appContext = getContext<AppViewerContext>('AppViewerContext')

	$: componentInput && appContext?.onchange?.()
	let s3FileUploadRawMode = false
	let s3FilePicker: S3FilePicker | undefined = undefined
</script>

{#key subFieldType}
	{#if componentInput?.type === 'static'}
		{#if fieldType === 'number' || fieldType === 'integer'}
			<input on:keydown|stopPropagation type="number" bind:value={componentInput.value} />
		{:else if fieldType === 'textarea'}
			<textarea use:autosize on:keydown|stopPropagation bind:value={componentInput.value}
			></textarea>
		{:else if fieldType === 'date'}
			<input on:keydown|stopPropagation type="date" bind:value={componentInput.value} />
		{:else if fieldType === 'time'}
			<input on:keydown|stopPropagation type="time" bind:value={componentInput.value} />
		{:else if fieldType === 'datetime'}
			<DateTimeInput bind:value={componentInput.value} />
		{:else if fieldType === 'boolean'}
			<Toggle bind:checked={componentInput.value} size="xs" class="mt-2" />
		{:else if fieldType === 'select' && selectOptions}
			{#if subFieldType === 'db-table'}
				<DBTableSelect bind:componentInput {selectOptions} {id} />
			{:else}
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
			{/if}
		{:else if fieldType === 'icon-select'}
			<IconSelectInput bind:value={componentInput.value} />
		{:else if fieldType === 'tab-select'}
			<TabSelectInput bind:componentInput />
		{:else if fieldType === 'resource' && subFieldType && ['mysql', 'postgres', 'ms_sql_server', 'snowflake', 'snowflake_oauth', 'bigquery', 'oracledb'].includes(subFieldType)}
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
				resourceType={subFieldType === 'postgres' ? 'postgresql' : subFieldType}
			/>
		{:else if fieldType === 'resource' && subFieldType === 's3'}
			<ResourcePicker
				placeholder="S3 resource (workspace s3 if empty)"
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
						variant="border"
						color="light"
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
					on:close={(e) => {
						if (e.detail) {
							if (componentInput) {
								componentInput.value = e.detail
							}
							s3FileUploadRawMode = true
						}
					}}
				/>
			{:else if format?.startsWith('resource-') && (componentInput.value == undefined || typeof componentInput.value == 'string')}
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
		{:else if fieldType === 'app-path'}
			<AppPicker bind:value={componentInput.value} />
		{:else}
			<div class="flex gap-1 relative w-full">
				<textarea
					rows="1"
					use:autosize
					on:keydown|stopPropagation
					placeholder={placeholder ?? 'Static value'}
					bind:value={componentInput.value}
					class="!pr-12"
				></textarea>
			</div>
		{/if}
	{/if}
{/key}
