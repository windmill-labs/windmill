<script lang="ts">
	import { run } from 'svelte/legacy'

	import type { Schema } from '$lib/common'
	import { ResourceService, type Resource, type ResourceType } from '$lib/gen'
	import { canWrite, emptyString, isOwner, urlize } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { Alert, Skeleton } from './common'
	import Path from './Path.svelte'
	import Required from './Required.svelte'

	import { userStore, workspaceStore } from '$lib/stores'
	import SchemaForm from './SchemaForm.svelte'
	import SimpleEditor from './SimpleEditor.svelte'
	import Toggle from './Toggle.svelte'
	import { sendUserToast } from '$lib/toast'
	import TestConnection from './TestConnection.svelte'
	import { Pen } from 'lucide-svelte'
	import Markdown from 'svelte-exmarkdown'
	import autosize from '$lib/autosize'
	import GfmMarkdown from './GfmMarkdown.svelte'
	import TestTriggerConnection from './triggers/TestTriggerConnection.svelte'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'

	interface Props {
		canSave?: boolean
		resource_type?: string | undefined
		path?: string
		newResource?: boolean
		hidePath?: boolean
		watchChanges?: boolean
		defaultValues?: Record<string, any> | undefined
	}

	let {
		canSave = $bindable(true),
		resource_type = $bindable(undefined),
		path = $bindable(''),
		newResource = false,
		hidePath = false,
		watchChanges = false,
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
	const dispatchIfMounted = createDispatcherIfMounted(dispatch)

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
				if (resourceTypeInfo?.format_extension) {
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
			if (resourceTypeInfo?.format_extension) {
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
		canSave = can_write && isValid && jsonError == ''
	})
	run(() => {
		watchChanges && dispatchIfMounted('change', { path, args, description })
	})
	run(() => {
		rawCode && parseJson()
	})
	run(() => {
		linkedVars.length > 0 && path && updateArgsFromLinkedVars()
	})
	run(() => {
		textFileContent && parseTextFileContent()
	})
</script>

<div>
	<div class="flex flex-col gap-3 py-3">
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
			<h4 class="mt-4 mb-2">{resourceTypeInfo?.name} description</h4>
			<div class="text-sm">
				<Markdown md={urlize(resourceTypeInfo?.description ?? '', 'md')} />
			</div>
		{/if}
		<h4 class="mt-4 inline-flex items-center gap-4"
			>Resource description <Required required={false} />
			{#if can_write}
				<div class="flex gap-1 items-center">
					<Toggle size="xs" bind:checked={editDescription} />
					<Pen size={14} />
				</div>
			{/if}</h4
		>
		{#if can_write && editDescription}
			<div>
				<div class="flex flex-row-reverse text-2xs text-tertiary -mt-1">GH Markdown</div>
				<textarea
					disabled={!can_write}
					use:autosize
					bind:value={description}
					placeholder={DESCRIPTION_PLACEHOLDER}
				></textarea>
			</div>
		{:else if description == undefined || description == ''}
			<div class="text-sm text-tertiary">No description provided</div>
		{:else}
			<div class="mt-2"></div>

			<GfmMarkdown md={description} />
		{/if}
		<div class="flex w-full justify-between items-center mt-4">
			<div></div>
			{#if resourceToEdit?.resource_type === 'nats' || resourceToEdit?.resource_type === 'kafka'}
				<TestTriggerConnection kind={resourceToEdit?.resource_type} args={{ connection: args }} />
			{:else}
				<TestConnection resourceType={resourceToEdit?.resource_type} {args} />
			{/if}
			<Toggle
				on:change={(e) => switchTab(e.detail)}
				options={{
					right: 'As JSON'
				}}
			/>
		</div>
		<div>
			{#if loadingSchema}
				<Skeleton layout={[[4]]} />
			{:else if !viewJsonSchema && resourceSchema && resourceSchema?.properties}
				{#if resourceTypeInfo?.format_extension}
					<h5 class="mt-4 inline-flex items-center gap-4 pb-2">
						File content ({resourceTypeInfo.format_extension})
					</h5>
					<div class="h-full w-full border p-1 rounded">
						<SimpleEditor
							autoHeight
							class="editor"
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
				<div class="h-full w-full border p-1 rounded">
					<SimpleEditor
						autoHeight
						class="editor"
						lang="json"
						bind:code={rawCode}
						fixedOverflowWidgets={false}
					/>
				</div>
			{/if}
		</div>
	</div>
</div>
