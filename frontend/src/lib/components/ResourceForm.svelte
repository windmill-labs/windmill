<script lang="ts">
	import ResourceDescriptionField from './ResourceDescriptionField.svelte'
	import type { Schema } from '$lib/common'
	import type { Resource, ResourceType } from '$lib/gen'
	import { onDestroy, untrack } from 'svelte'
	import { setEditorUnparseable } from './pendingEditorFlush'
	import { emptyString, isOwner, urlize } from '$lib/utils'
	import { Alert, Button, Skeleton } from './common'
	import { Trash2 } from 'lucide-svelte'
	import Path from './Path.svelte'
	import LabelsInput from './LabelsInput.svelte'
	import { enterpriseLicense, type UserExt } from '$lib/stores'
	import SchemaForm from './SchemaForm.svelte'
	import SimpleEditor from './SimpleEditor.svelte'
	import FilesetEditor from './FilesetEditor.svelte'
	import Toggle from './Toggle.svelte'
	import TestConnection from './TestConnection.svelte'
	import TestAiKey from './copilot/TestAIKey.svelte'
	import { aiResourceProvider, isTokenVertexResource } from './copilot/aiResourceProvider'
	import { copilotInfo } from '$lib/aiStore'
	import GfmMarkdown from './GfmMarkdown.svelte'
	import TestTriggerConnection from './triggers/TestTriggerConnection.svelte'
	import GitHubAppIntegration from './GitHubAppIntegration.svelte'
	import GitLabIntegration from './GitLabIntegration.svelte'
	import ResourceGen from './copilot/ResourceGen.svelte'
	import SyncResourceTypes from './SyncResourceTypes.svelte'
	import Label from './Label.svelte'
	import ResourcePathHint from './ResourcePathHint.svelte'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import DropdownV2 from './DropdownV2.svelte'
	import ResourceValueReference, {
		canReferenceWholeValue,
		resourceValueMenuItems,
		resourceValueReference
	} from './ResourceValueReference.svelte'

	const operatingWorkspace = useOperatingWorkspace()

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
		/** `undefined` while the acting user or the resource is still being resolved: neither a
		 * grant nor the denial the read-only alert announces. */
		can_write: boolean | undefined
		resource_type: string | undefined
		resourceTypeInfo: ResourceType | undefined
		resourceSchema: Schema | undefined
		loadingSchema: boolean
		resourceToEdit: Resource | undefined
		onLoadResourceType?: () => void
		/** Workspace the path is validated against and the connection is tested in;
		 * defaults to the nav workspace. */
		workspace?: string | undefined
		/** The user acting in `workspace`, resolved by the editor above. `undefined` while
		 * `null` while that lookup is pending or after it failed: every check below then
		 * refuses, rather than answering with the navigation user's rights in another
		 * workspace. */
		actingUser: UserExt | null
		/** Fired once the GitLab picker has stored the picked project's token, so a
		 * form that would otherwise file the URL as a secret knows it holds none. */
		onCredentialStored?: () => void
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
		onLoadResourceType,
		workspace = undefined,
		actingUser,
		onCredentialStored
	}: Props = $props()

	let ws = $derived(workspace ?? $operatingWorkspace)
	let aiProvider = $derived(aiResourceProvider(resource_type, args))

	let rawCode: string | undefined = $state(undefined)
	let textFileContent: string = $state('')

	let reference = $derived(resourceValueReference(args))
	let rawMode = $derived(
		viewJsonSchema ||
			(!resourceTypeInfo?.is_fileset && !(resourceSchema && resourceSchema.properties))
	)
	// Pinned when the value arrives as a reference (loaded or picked), not when the JSON editor's
	// text parses to one: that editor stays mounted while it is being typed into. Any other form
	// would replace the string with its own fields, so it never renders over a reference.
	let referencePinned = $state(untrack(() => !!resourceValueReference(args)))
	let showReference = $derived(!!reference && (referencePinned || !rawMode))
	let valueReference: ResourceValueReference | undefined = $state(undefined)

	function setWholeValue(v: Record<string, any> | string) {
		args = v as Record<string, any>
		referencePinned = typeof v === 'string'
		jsonError = ''
		// Reseeded from the new value by the effect below once the raw editor is the input again.
		rawCode = undefined
	}

	$effect(() => {
		if (showReference) isValid = true
	})

	// Keys the schema form cannot render (e.g. a property later removed from the resource type)
	// still ship with the value, and a `$var:` among them fails the job once the variable is gone.
	let orphanKeys = $derived.by(() => {
		const props = resourceSchema?.properties
		if (
			!props ||
			showReference ||
			rawMode ||
			loadingSchema ||
			resourceTypeInfo?.is_fileset ||
			resourceTypeInfo?.format_extension ||
			!args ||
			typeof args !== 'object' ||
			Array.isArray(args)
		) {
			return []
		}
		return Object.keys(args).filter((k) => !(k in props))
	})

	// Rendered untyped, which the form shows as a JSON input, so any stored value fits.
	let formSchema = $derived(
		orphanKeys.length > 0 && resourceSchema
			? {
					...resourceSchema,
					properties: {
						...resourceSchema.properties,
						...Object.fromEntries(orphanKeys.map((k) => [k, {}]))
					}
				}
			: resourceSchema
	)

	let orphanWarnings = $derived(
		Object.fromEntries(
			orphanKeys.map((k) => [
				k,
				'Not in the resource type schema, but still passed to scripts using this resource. Remove it if it is no longer used.'
			])
		)
	)

	function removeOrphanKey(key: string) {
		const { [key]: _, ...rest } = args
		args = rest
	}

	// This field is a bare SimpleEditor parsed here, so it never passes through JsonEditor —
	// it has to register itself, or a caller persisting what is on screen would save the
	// last value that parsed and leave without the text in front of the user.
	const unparseableKey = {}
	onDestroy(() => setEditorUnparseable(unparseableKey, false))

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

	// The raw JSON editor is the active input whenever the "As JSON" toggle is on,
	// or no schema-based form can be rendered (e.g. the resource type is missing
	// from the workspace). In both cases rawCode must be seeded from args.
	let usesRawEditor = $derived(!loadingSchema && !showReference && rawMode)

	$effect(() => {
		if (rawCode !== undefined) parseJson()
	})

	// Both halves, and from the current parse rather than from a transition: `rawCode`
	// outlives the raw editor, so text that does not parse is the user's to fix exactly
	// while that editor is the active input — which the schema loading and the resource
	// type flip as well as the toggle, and only the toggle reseeds `rawCode`.
	$effect(() => {
		setEditorUnparseable(unparseableKey, usesRawEditor && jsonError !== '')
	})

	$effect(() => {
		if (usesRawEditor && rawCode === undefined) {
			rawCode = JSON.stringify(args, null, 2)
		}
	})

	// Seed the JSON editor when the resource type schema is missing
	// (restores the old ResourceEditor's catch-block behavior)
	$effect(() => {
		if (
			resource_type &&
			!loadingSchema &&
			!resourceSchema &&
			!showReference &&
			rawCode === undefined
		) {
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

{#if !emptyString(resourceTypeInfo?.description)}
	<GfmMarkdown md={urlize(resourceTypeInfo?.description ?? '', 'md')} prose="sm" noPadding />
{/if}

{#if !hidePath}
	<div>
		{#if can_write === false}
			<div class="my-2">
				<Alert type="warning" title="Only read access">
					You only have read access to this resource and cannot edit it
				</Alert>
			</div>
		{/if}
		<Label label="Path">
			<ResourcePathHint />
			<Path
				disabled={initialPath != '' && !isOwner(initialPath, actingUser ?? undefined, ws)}
				bind:path
				{initialPath}
				namePlaceholder="resource"
				kind="resource"
				workspaceOverride={workspace}
				{actingUser}
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

<ResourceDescriptionField bind:description canWrite={can_write} />

<div class="flex flex-col gap-1">
	<div class="w-full flex gap-4 flex-row-reverse items-center">
		{#if can_write && (showReference || canReferenceWholeValue(resourceTypeInfo))}
			<DropdownV2
				placement="bottom-end"
				items={() =>
					resourceValueMenuItems(showReference ? reference : undefined, {
						pickVariable: () => valueReference?.pickVariable(),
						editFields: () => setWholeValue({})
					})}
			/>
		{/if}
		{#if !showReference}
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
				<TestConnection
					resourceType={resourceToEdit?.resource_type}
					{args}
					workspaceOverride={workspace}
				/>
			{/if}
			{#if aiProvider}
				<TestAiKey
					{aiProvider}
					workspace={ws}
					resourceValue={args}
					model={$copilotInfo.aiModels.find((m) => m.provider === aiProvider)?.model}
				/>
			{/if}
			{#if resource_type === 'git_repository' && ws && (actingUser?.is_admin || actingUser?.is_super_admin)}
				<GitHubAppIntegration
					resourceType={resource_type}
					{args}
					{description}
					onArgsUpdate={(newArgs) => {
						args = newArgs
						// The raw editor is also what a workspace missing the resource type
						// gets, and it holds its own copy of the value: without this the
						// picker fills in a URL nothing on screen ever shows.
						if (viewJsonSchema || !resourceSchema) {
							rawCode = JSON.stringify(args, null, 2)
						}
					}}
					onDescriptionUpdate={(newDescription) => (description = newDescription)}
				/>
				<GitLabIntegration
					resourceType={resource_type}
					{args}
					workspace={ws}
					{onCredentialStored}
					onArgsUpdate={(newArgs) => {
						args = newArgs
						// The raw editor is also what a workspace missing the resource type
						// gets, and it holds its own copy of the value: without this the
						// picker fills in a URL nothing on screen ever shows.
						if (viewJsonSchema || !resourceSchema) {
							rawCode = JSON.stringify(args, null, 2)
						}
					}}
				/>
			{/if}
		{/if}
	</div>
	{#if $enterpriseLicense && !showReference && isTokenVertexResource(resource_type, args)}
		<p class="text-xs text-secondary">
			To stop storing and refreshing an access token, use a
			<span class="font-mono">vertex_ai_federated</span> resource instead: Windmill authenticates to
			Vertex AI through Workload Identity Federation and refreshes the token itself.
		</p>
	{/if}

	<div>
		<ResourceValueReference
			bind:this={valueReference}
			reference={showReference ? reference : undefined}
			disabled={!can_write}
			workspace={ws}
			onPick={setWholeValue}
		/>
		{#if showReference}{:else if loadingSchema}
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
					schema={formSchema}
					bind:args
					bind:isValid
					schemaFieldWarnings={orphanWarnings}
					{workspace}
				>
					{#snippet actions({ item })}
						{#if can_write && orphanKeys.includes(item.value)}
							<Button
								variant="subtle"
								unifiedSize="sm"
								iconOnly
								startIcon={{ icon: Trash2 }}
								title="Remove from this resource"
								onclick={() => removeOrphanKey(item.value)}
							/>
						{/if}
					{/snippet}
				</SchemaForm>
			{/if}
		{:else if !can_write}
			<input type="text" disabled value={rawCode} />
		{:else}
			{#if !viewJsonSchema && !resourceSchema}
				<div class="flex flex-col gap-2 mb-4">
					<p class="text-red-500 dark:text-red-400 text-xs">
						Resource type '{resource_type}' not found in your workspace
					</p>
					<SyncResourceTypes resourceType={resource_type} onSynced={() => onLoadResourceType?.()} />
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
