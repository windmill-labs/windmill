<script lang="ts">
	import type { Schema } from '$lib/common'
	import type { Resource, ResourceType } from '$lib/gen'
	import { emptyString, isOwner, urlize } from '$lib/utils'
	import { Alert, Skeleton } from './common'
	import Path from './Path.svelte'
	import LabelsInput from './LabelsInput.svelte'
	import Required from './Required.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import SchemaForm from './SchemaForm.svelte'
	import SimpleEditor from './SimpleEditor.svelte'
	import FilesetEditor from './FilesetEditor.svelte'
	import Toggle from './Toggle.svelte'
	import TestConnection from './TestConnection.svelte'
	import { Pen } from 'lucide-svelte'
	import Markdown from 'svelte-exmarkdown'
	import autosize from '$lib/autosize'
	import GfmMarkdown from './GfmMarkdown.svelte'
	import TestTriggerConnection from './triggers/TestTriggerConnection.svelte'
	import GitHubAppIntegration from './GitHubAppIntegration.svelte'
	import Button from './common/button/Button.svelte'
	import ResourceGen from './copilot/ResourceGen.svelte'
	import SyncResourceTypes from './SyncResourceTypes.svelte'
	import Label from './Label.svelte'

	interface Props {
		path: string
		initialPath: string
		hidePath?: boolean
		labels: string[] | undefined
		description: string
		args: Record<string, any>
		wsSpecific: boolean
		isValid: boolean
		viewJsonSchema: boolean
		jsonError: string
		deployTo: string | undefined
		can_write: boolean
		resource_type: string | undefined
		resourceTypeInfo: ResourceType | undefined
		resourceSchema: Schema | undefined
		loadingSchema: boolean
		resourceToEdit: Resource | undefined
		onLoadResourceType?: () => void
	}

	let {
		path = $bindable(),
		initialPath,
		hidePath = false,
		labels = $bindable(),
		description = $bindable(),
		args = $bindable(),
		wsSpecific = $bindable(),
		isValid = $bindable(),
		viewJsonSchema = $bindable(),
		jsonError = $bindable(),
		deployTo,
		can_write,
		resource_type,
		resourceTypeInfo,
		resourceSchema,
		loadingSchema,
		resourceToEdit,
		onLoadResourceType
	}: Props = $props()

	let editDescription = $state(false)
	let rawCode: string | undefined = $state(undefined)
	let textFileContent: string = $state('')

	function parseJson() {
		try {
			args = JSON.parse(rawCode ?? '')
			jsonError = ''
		} catch (e) {
			jsonError = e.message
		}
	}

	function parseTextFileContent() {
		args = { content: textFileContent }
	}

	$effect(() => {
		if (rawCode !== undefined) parseJson()
	})

	$effect(() => {
		if (viewJsonSchema && rawCode === undefined) {
			rawCode = JSON.stringify(args, null, 2)
		}
	})

	$effect(() => {
		if (textFileContent) parseTextFileContent()
	})

	$effect(() => {
		if (resourceTypeInfo?.format_extension && !resourceTypeInfo?.is_fileset && !viewJsonSchema) {
			textFileContent = args?.content ?? ''
		}
	})
</script>

{#if !hidePath}
	<div>
		{#if !can_write}
			<div class="my-2">
				<Alert type="warning" title="Only read access">
					You only have read access to this resource and cannot edit it
				</Alert>
			</div>
		{/if}
		<Label label="Path">
			<Path
				disabled={initialPath != '' && !isOwner(initialPath, $userStore, $workspaceStore)}
				bind:path
				{initialPath}
				namePlaceholder="resource"
				kind="resource"
			/>
		</Label>
	</div>
{/if}
<LabelsInput bind:labels class="-mt-4" />

{#if deployTo}
	<Label
		label="Workspace specific"
		tooltip="Prevents this resource from being deployed to prod/staging. When enabled, any variable referenced via $var: inside the resource value is also automatically marked workspace-specific. Disabling this toggle does not un-mark those variables — they may be referenced by other resources."
	>
		<Toggle bind:checked={wsSpecific} />
	</Label>
{/if}

{#if !emptyString(resourceTypeInfo?.description)}
	<div class="flex flex-col gap-1">
		<h4 class="text-xs text-emphasis font-semibold">{resourceTypeInfo?.name} description</h4>
		<div class="text-xs text-primary font-normal">
			<Markdown md={urlize(resourceTypeInfo?.description ?? '', 'md')} />
		</div>
	</div>
{/if}

<div class="flex flex-col gap-1">
	<h4 class="inline-flex items-center gap-2 text-xs text-emphasis font-semibold"
		>Resource description <Required required={false} />
		{#if can_write}
			<Button
				variant="subtle"
				unifiedSize="xs"
				btnClasses={editDescription ? 'bg-surface-hover' : ''}
				startIcon={{ icon: Pen }}
				on:click={() => (editDescription = !editDescription)}
			/>
		{/if}
	</h4>
	{#if can_write && editDescription}
		<div class="relative">
			<div class="text-2xs text-primary absolute -top-4 right-0">GH Markdown</div>
			<textarea
				class="text-xs text-primary font-normal"
				disabled={!can_write}
				use:autosize
				bind:value={description}
				placeholder="Describe what this resource is for"
			></textarea>
		</div>
	{:else if description == undefined || description == ''}
		<div class="text-xs text-secondary font-normal">No description provided</div>
	{:else}
		<div class="text-xs text-primary font-normal">
			<GfmMarkdown md={description} noPadding />
		</div>
	{/if}
</div>

<div class="flex flex-col gap-1">
	<div class="w-full flex gap-4 flex-row-reverse items-center">
		<Toggle
			bind:checked={viewJsonSchema}
			on:change={(e) => {
				if (e.detail) {
					rawCode = JSON.stringify(args, null, 2)
				} else if (resourceTypeInfo?.format_extension && !resourceTypeInfo?.is_fileset) {
					textFileContent = args?.content ?? ''
				}
			}}
			options={{
				right: 'As JSON'
			}}
		/>
		<ResourceGen
			bind:args
			resourceType={resource_type}
			resourceName={path}
			resourceDescription={description}
			{resourceSchema}
		/>
		{#if resourceToEdit?.resource_type === 'nats' || resourceToEdit?.resource_type === 'kafka'}
			<TestTriggerConnection kind={resourceToEdit?.resource_type} args={{ connection: args }} />
		{:else}
			<TestConnection resourceType={resourceToEdit?.resource_type} {args} />
		{/if}
		{#if resource_type === 'git_repository' && $workspaceStore && ($userStore?.is_admin || $userStore?.is_super_admin)}
			<GitHubAppIntegration
				resourceType={resource_type}
				{args}
				{description}
				onArgsUpdate={(newArgs) => {
					args = newArgs
					if (viewJsonSchema) {
						rawCode = JSON.stringify(args, null, 2)
					}
				}}
				onDescriptionUpdate={(newDescription) => (description = newDescription)}
			/>
		{/if}
	</div>

	<div>
		{#if loadingSchema}
			<Skeleton layout={[[4]]} />
		{:else if !viewJsonSchema && resourceTypeInfo?.is_fileset}
			<div class="mt-1 flex items-center gap-2">
				<h5 class="inline-flex items-center gap-4">Fileset</h5>
				<ResourceGen
					bind:args
					resourceType={resource_type}
					resourceName={path}
					resourceDescription={description}
					{resourceSchema}
					isFileset
				/>
			</div>
			<FilesetEditor bind:args />
		{:else if !viewJsonSchema && resourceSchema && resourceSchema?.properties}
			{#if resourceTypeInfo?.format_extension}
				<h5 class="mt-1 inline-flex items-center gap-4">
					File content ({resourceTypeInfo.format_extension})
				</h5>
				<div class="">
					<SimpleEditor
						autoHeight
						lang={resourceTypeInfo.format_extension}
						bind:code={textFileContent}
						fixedOverflowWidgets={false}
					/>
				</div>
			{:else}
				<SchemaForm
					onlyMaskPassword
					noDelete
					disabled={!can_write}
					compact
					schema={resourceSchema}
					bind:args
					bind:isValid
				/>
			{/if}
		{:else if !can_write}
			<input type="text" disabled value={rawCode} />
		{:else}
			{#if !viewJsonSchema}
				<div class="flex flex-col gap-2 mb-4">
					<p class="text-red-500 dark:text-red-400 text-xs">
						Resource type '{resource_type}' not found in your workspace
					</p>
					<SyncResourceTypes onSynced={() => onLoadResourceType?.()} />
					<p class="italic text-secondary text-xs"> Define the value in JSON directly </p>
				</div>
			{/if}

			{#if !emptyString(jsonError)}<span class="text-red-400 text-xs mb-1 flex flex-row-reverse"
					>{jsonError}</span
				>{:else}<div class="py-2"></div>{/if}
			<div class="bg-surface-tertiary rounded-md border py-2.5">
				<SimpleEditor autoHeight lang="json" bind:code={rawCode} />
			</div>
		{/if}
	</div>
</div>
