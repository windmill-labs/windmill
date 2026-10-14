<script lang="ts">
	import { OauthService, type ResourceType } from '$lib/gen'
	import FilesetEditor from './FilesetEditor.svelte'
	import { emptySchema, emptyString } from '$lib/utils'
	import SchemaForm from './SchemaForm.svelte'
	import Toggle from './Toggle.svelte'
	import TestConnection from './TestConnection.svelte'
	import TestAiKey from './copilot/TestAIKey.svelte'
	import { aiResourceProvider } from './copilot/aiResourceProvider'
	import { copilotInfo } from '$lib/aiStore'
	import Popover from './meltComponents/Popover.svelte'
	import Button from './common/button/Button.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { untrack } from 'svelte'
	import GitHubAppIntegration from './GitHubAppIntegration.svelte'
	import GitLabIntegration from './GitLabIntegration.svelte'
	import BedrockCredentialsCheck from './BedrockCredentialsCheck.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import ResourceGen from './copilot/ResourceGen.svelte'
	import SyncResourceTypes from './SyncResourceTypes.svelte'
	import { parsePostgresConnectionString } from '$lib/utils/postgresConnectionString'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'

	const operatingWorkspace = useOperatingWorkspace()

	interface Props {
		resourceType: string
		resourceTypeInfo: ResourceType | undefined
		args?: Record<string, any> | any
		linkedSecrets?: string[]
		isValid?: boolean
		linkedSecretCandidates?: string[] | undefined
		description?: string | undefined
		/** Workspace the resource is being saved into, which is not always the one
		 * being navigated. The GitLab picker has to store the credential where the
		 * resource will look for it. */
		workspace?: string
		/** Fired once the GitLab picker has stored the picked project's token, so a
		 * form that would otherwise file the URL as a secret knows it holds none. */
		onCredentialStored?: () => void
		onSynced?: () => void
	}

	let {
		resourceType,
		resourceTypeInfo,
		args = $bindable({}),
		linkedSecrets = $bindable([]),
		isValid = $bindable(true),
		linkedSecretCandidates = undefined,
		description = $bindable(undefined),
		workspace = undefined,
		onCredentialStored,
		onSynced = undefined
	}: Props = $props()

	let aiProvider = $derived(aiResourceProvider(resourceType))

	let schema = $state(emptySchema())
	let notFound = $state(false)

	let supabaseWizard = $state(false)

	async function isSupabaseAvailable() {
		try {
			supabaseWizard = ((await OauthService.listOauthConnects()) ?? []).some(
				(c) => c.name === 'supabase_wizard'
			)
		} catch (error) {}
	}
	async function loadSchema() {
		if (!resourceTypeInfo) return
		rawCode = '{}'
		viewJsonSchema = false
		try {
			schema = resourceTypeInfo.schema as any
			// A resource type may declare no properties at all — `dbt_profile` is a
			// `profiles.yml` block whose keys are its adapter's, not Windmill's. That
			// is a JSON-edited type, NOT a missing one: `Object.keys(undefined)` threw
			// into the catch below, so the drawer told the user to sync a type it had.
			schema.order = schema.order ?? Object.keys(schema.properties ?? {}).sort()
			notFound = false
		} catch (e) {
			notFound = true
		}
	}

	function parseJson() {
		try {
			args = JSON.parse(rawCode)
			error = ''
			isValid = true
		} catch (e) {
			isValid = false
			error = e.message
		}
	}
	let error = $state('')
	let rawCode = $state('')
	let viewJsonSchema = $state(false)

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

	let connectionString = $state('')
	let validConnectionString = $state(true)
	function parseConnectionString(close: (_: any) => void) {
		const parts = parsePostgresConnectionString(connectionString)
		if (!parts) {
			validConnectionString = false
			return
		}
		validConnectionString = true
		rawCode = JSON.stringify(
			{
				...args,
				user: parts.user,
				password: parts.password || args?.password,
				host: parts.host,
				port: parts.port || args?.port,
				dbname: parts.dbname || args?.dbname,
				sslmode: parts.sslmode || args?.sslmode,
				options: parts.options || args?.options
			},
			null,
			2
		)
		rawCodeEditor?.setCode(rawCode)
		close(null)
	}

	let rawCodeEditor: { setCode: (code: string) => void } | undefined = $state(undefined)
	let textFileContent: string | undefined = $state(undefined)

	function applySupabasePick(value: Record<string, any>) {
		args = { ...(args ?? {}), ...value }
		rawCode = JSON.stringify(args, null, 2)
		rawCodeEditor?.setCode(rawCode)
	}

	function parseTextFileContent() {
		args = {
			content: textFileContent
		}
	}
	$effect(() => {
		$operatingWorkspace && untrack(() => loadSchema())
	})
	$effect(() => {
		notFound && rawCode && untrack(() => parseJson())
	})
	$effect(() => {
		rawCode && untrack(() => parseJson())
	})
	$effect(() => {
		textFileContent && untrack(() => parseTextFileContent())
	})
	$effect(() => {
		resourceType == 'postgresql' && untrack(() => isSupabaseAvailable())
	})
</script>

{#if !notFound}
	<div class="w-full flex gap-2 flex-row-reverse items-center">
		<Toggle
			on:change={(e) => switchTab(e.detail)}
			options={{
				right: 'As JSON'
			}}
			class="as-json-toggle"
		/>
		<ResourceGen
			bind:args
			{resourceType}
			resourceSchema={notFound ? undefined : schema}
			isFileset={resourceTypeInfo?.is_fileset ?? false}
		/>
		<TestConnection {resourceType} {args} />
		{#if aiProvider}
			<TestAiKey
				{aiProvider}
				workspace={workspace ?? $operatingWorkspace}
				resourceValue={args}
				model={$copilotInfo.aiModels.find((m) => m.provider === aiProvider)?.model}
			/>
		{/if}
		{#if resourceType == 'postgresql'}
			<Popover
				floatingConfig={{
					placement: 'bottom'
				}}
			>
				{#snippet trigger()}
					<Button spacingSize="sm" size="xs" unifiedSize="md" variant="default" nonCaptureEvent>
						From connection string
					</Button>
				{/snippet}
				{#snippet content({ close })}
					<div class="block text-primary p-4">
						<div class="w-[550px] flex flex-col items-start gap-1">
							<div class="flex flex-row gap-1 w-full">
								<input
									type="text"
									bind:value={connectionString}
									placeholder="postgres://user:password@host:5432/dbname?sslmode=disable"
								/>
								<Button
									size="xs"
									color="blue"
									buttonType="button"
									on:click={() => {
										parseConnectionString(close)
									}}
									disabled={connectionString.length <= 0}
								>
									Apply
								</Button>
							</div>
							{#if !validConnectionString}
								<p class="text-red-500 text-xs">Could not parse connection string</p>
							{/if}
						</div>
					</div>
				{/snippet}
			</Popover>
		{/if}
		{#if resourceType == 'postgresql' && supabaseWizard}
			<!-- Imported here rather than at the top so the wizard's Supabase graph stays out of
			this form's chunk, which loads on the resources page and in every resource drawer. -->
			{#await import('./workspaceSettings/SupabaseResourceConnect.svelte')}
				<Loader2 class="animate-spin" />
			{:then Module}
				<Module.default onPicked={applySupabasePick} />
			{/await}
		{/if}
		<GitHubAppIntegration
			{resourceType}
			{args}
			{description}
			onArgsUpdate={(newArgs) => {
				args = newArgs
				rawCode = JSON.stringify(args, null, 2)
				rawCodeEditor?.setCode(rawCode)
			}}
			onDescriptionUpdate={(newDescription) => (description = newDescription)}
		/>
		<!-- Last in a `flex-row-reverse` row, so it lands beside the GitHub App
		button without splitting it from its own refresh control. -->
		<GitLabIntegration
			{resourceType}
			{args}
			{workspace}
			{onCredentialStored}
			onArgsUpdate={(newArgs) => {
				args = newArgs
				rawCode = JSON.stringify(args, null, 2)
				rawCodeEditor?.setCode(rawCode)
			}}
		/>
	</div>
	{#if resourceType?.includes('bedrock') && !isCloudHosted()}
		<BedrockCredentialsCheck />
	{/if}
{:else}
	<p class="text-primary font-normal text-xs mb-4"
		>No corresponding resource type found in your workspace for {resourceType}. Define the value in
		JSON directly</p
	>
	<SyncResourceTypes {resourceType} {onSynced} />
{/if}
{#if notFound || viewJsonSchema || !schema?.properties}
	{#if !emptyString(error)}<span class="text-red-400 text-xs mb-1 flex flex-row-reverse"
			>{error}</span
		>{:else}<div class="py-2"></div>{/if}
	<div class="h-full w-full border p-1 rounded">
		{#await import('$lib/components/SimpleEditor.svelte')}
			<Loader2 class="animate-spin" />
		{:then Module}
			<Module.default
				bind:this={rawCodeEditor}
				autoHeight
				lang="json"
				bind:code={rawCode}
				fixedOverflowWidgets={false}
			/>
		{/await}
	</div>
{:else if resourceTypeInfo?.is_fileset}
	<h5 class="mt-1 inline-flex items-center gap-4"> Fileset </h5>
	<FilesetEditor bind:args />
{:else if resourceTypeInfo?.format_extension}
	<h5 class="mt-4 inline-flex items-center gap-4">
		File content ({resourceTypeInfo.format_extension})
	</h5>
	<div class="py-2"></div>
	<div class="h-full w-full border p-1 rounded">
		{#await import('$lib/components/SimpleEditor.svelte')}
			<Loader2 class="animate-spin" />
		{:then Module}
			<Module.default
				bind:this={rawCodeEditor}
				autoHeight
				lang={resourceTypeInfo.format_extension}
				bind:code={textFileContent}
				fixedOverflowWidgets={false}
			/>
		{/await}
	</div>
{:else}
	<SchemaForm
		onlyMaskPassword
		noDelete
		{linkedSecretCandidates}
		bind:linkedSecrets
		isValid
		{schema}
		bind:args
	/>
{/if}
