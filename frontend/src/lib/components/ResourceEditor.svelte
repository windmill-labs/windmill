<script lang="ts">
	import { run } from 'svelte/legacy'

	import type { Schema } from '$lib/common'
	import { ResourceService, type Resource, type ResourceType } from '$lib/gen'
	import { canWrite, emptyString, isOwner, urlize } from '$lib/utils'
	import { createEventDispatcher, untrack } from 'svelte'
	import { Alert, Skeleton } from './common'
	import Path from './Path.svelte'
	import Required from './Required.svelte'

	import { userStore, workspaceStore } from '$lib/stores'
	import SchemaForm from './SchemaForm.svelte'
	import SimpleEditor from './SimpleEditor.svelte'
	import FilesetEditor from './FilesetEditor.svelte'
	import Toggle from './Toggle.svelte'
	import { sendUserToast } from '$lib/toast'
	import TestConnection from './TestConnection.svelte'
	import { Pen } from 'lucide-svelte'
	import Markdown from 'svelte-exmarkdown'
	import autosize from '$lib/autosize'
	import GfmMarkdown from './GfmMarkdown.svelte'
	import TestTriggerConnection from './triggers/TestTriggerConnection.svelte'
	import GitHubAppIntegration from './GitHubAppIntegration.svelte'
	import Button from './common/button/Button.svelte'
	import { clearJsonSchemaResourceCache } from './schema/jsonSchemaResource.svelte'

	interface Props {
		canSave?: boolean
		resource_type?: string | undefined
		path?: string
		newResource?: boolean
		hidePath?: boolean
		onChange?: (args: { path: string; args: Record<string, any>; description: string }) => void
		defaultValues?: Record<string, any> | undefined
	}

	let {
		canSave = $bindable(true),
		resource_type = $bindable(undefined),
		path = $bindable(''),
		newResource = false,
		hidePath = false,
		onChange,
		defaultValues = undefined
	}: Props = $props()

	let isValid = $state(true)
	let jsonError = $state('')
	let can_write = $state(true)

	let initialPath = path

	let resourceToEdit: Resource | undefined = $state(undefined)

	let description: string = $state('')
	let DESCRIPTION_PLACEHOLDER = `Describe what this resource is for`
	let resourceSchema: Schema | undefined = $state(undefined)
	let args: Record<string, any> = $state({})
	let loadingSchema = $state(true)
	let linkedVars: string[] = $state([])
	let resourceTypeInfo: ResourceType | undefined = $state(undefined)
	let editDescription = $state(false)
	let viewJsonSchema = $state(false)

	const dispatch = createEventDispatcher()

	let rawCode: string | undefined = $state(undefined)

	async function initEdit() {
		resourceToEdit = await ResourceService.getResource({ workspace: $workspaceStore!, path })
		description = resourceToEdit!.description ?? ''
		resource_type = resourceToEdit!.resource_type
		args = resourceToEdit?.value ?? ({} as any)
		loadResourceType()
		can_write =
			resourceToEdit.workspace_id == $workspaceStore &&
			canWrite(path, resourceToEdit.extra_perms ?? {}, $userStore)
		linkedVars = Object.entries(args)
			.filter(([_, v]) => typeof v == 'string' && v == `$var:${initialPath}`)
			.map(([k, _]) => k)
	}

	if (!newResource) {
		initEdit()
	} else if (resource_type) {
		loadResourceType()
	} else {
		sendUserToast('Resource type cannot be undefined for new resource creation', true)
	}

	export async function editResource(): Promise<void> {
		if (resourceToEdit) {
			await ResourceService.updateResource({
				workspace: $workspaceStore!,
				path: resourceToEdit.path,
				requestBody: { path, value: args, description }
			})
			if (resourceToEdit.resource_type === 'json_schema') {
				clearJsonSchemaResourceCache(resourceToEdit.path, $workspaceStore!)
			}
			sendUserToast(`Updated resource at ${path}`)
			dispatch('refresh', path)
		} else {
			throw Error('Cannot edit undefined resource')
		}
	}

	export async function createResource(): Promise<void> {
		await ResourceService.createResource({
			workspace: $workspaceStore!,
			requestBody: { path, value: args, description, resource_type: resource_type! }
		})
		sendUserToast(`Updated resource at ${path}`)
		dispatch('refresh', path)
	}

	async function loadResourceType(): Promise<void> {
		if (resource_type) {
			try {
				const resourceType = await ResourceService.getResourceType({
					workspace: $workspaceStore!,
					path: resource_type
				})

				resourceTypeInfo = resourceType
				if (resourceType.schema) {
					resourceSchema = resourceType.schema as Schema
					resourceSchema.order =
						resourceSchema.order ?? Object.keys(resourceSchema.properties).sort()
				}
				if (resourceTypeInfo?.format_extension && !resourceTypeInfo?.is_fileset) {
					textFileContent = args.content
				}
			} catch (err) {
				resourceSchema = undefined
				loadingSchema = false
				rawCode = JSON.stringify(args, null, 2)
			}
		} else {
			sendUserToast(`ResourceType cannot be undefined.`, true)
		}
		loadingSchema = false
	}

	function parseJson() {
		try {
			args = JSON.parse(rawCode ?? '')
			jsonError = ''
		} catch (e) {
			jsonError = e.message
		}
	}

	function updateArgsFromLinkedVars() {
		linkedVars.forEach((k) => {
			args[k] = `$var:${path}`
		})
	}

	let textFileContent: string = $state('')

	function switchTab(asJson: boolean) {
		viewJsonSchema = asJson
		if (asJson) {
			rawCode = JSON.stringify(args, null, 2)
		} else {
			parseJson()
			if (resourceTypeInfo?.format_extension && !resourceTypeInfo?.is_fileset) {
				textFileContent = args.content
			}
		}
	}

	function parseTextFileContent() {
		args = {
			content: textFileContent
		}
	}
	run(() => {
		if (defaultValues && Object.keys(defaultValues).length > 0) {
			args = defaultValues
		}
	})
	run(() => {
		canSave = (can_write && isValid && jsonError == '') || (viewJsonSchema && jsonError == '')
	})
	$effect(() => {
		onChange && onChange({ path, args, description })
	})
	run(() => {
		rawCode && untrack(() => parseJson())
	})
	run(() => {
		linkedVars.length > 0 && path && untrack(() => updateArgsFromLinkedVars())
	})
	run(() => {
		textFileContent && untrack(() => parseTextFileContent())
	})
</script>

<div>
	<div class="flex flex-col gap-6 py-2">
		{#if !hidePath}
			<div>
				{#if !can_write}
					<div class="m-2">
						<Alert type="warning" title="Only read access">
							You only have read access to this resource and cannot edit it
						</Alert>
					</div>
				{/if}
				<Path
					disabled={initialPath != '' && !isOwner(initialPath, $userStore, $workspaceStore)}
					bind:path
					{initialPath}
					namePlaceholder="resource"
					kind="resource"
				/>
			</div>
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
						placeholder={DESCRIPTION_PLACEHOLDER}
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
					on:change={(e) => switchTab(e.detail)}
					options={{
						right: 'As JSON'
					}}
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
							// Update rawCode if in JSON view mode
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
					<h5 class="mt-1 inline-flex items-center gap-4">
						Fileset
					</h5>
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
						<p class="italic text-secondary text-xs mb-4">
							No corresponding resource type found in your workspace for {resource_type}. Define the
							value in JSON directly
						</p>
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
	</div>
</div>
