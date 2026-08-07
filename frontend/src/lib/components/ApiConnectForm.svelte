<script lang="ts">
	import { OauthService, type ResourceType } from '$lib/gen'
	import FilesetEditor from './FilesetEditor.svelte'
	import { workspaceStore } from '$lib/stores'
	import { emptySchema, emptyString } from '$lib/utils'
	import SchemaForm from './SchemaForm.svelte'
	import Toggle from './Toggle.svelte'
	import TestConnection from './TestConnection.svelte'
	import SupabaseIcon from './icons/SupabaseIcon.svelte'
	import Popover from './meltComponents/Popover.svelte'
	import Button from './common/button/Button.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { untrack } from 'svelte'
	import GitHubAppIntegration from './GitHubAppIntegration.svelte'
	import BedrockCredentialsCheck from './BedrockCredentialsCheck.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import ResourceGen from './copilot/ResourceGen.svelte'
	import SyncResourceTypes from './SyncResourceTypes.svelte'
	import Modal2 from './common/modal/Modal2.svelte'
	import SupabaseProjectStep, {
		type SupabasePick
	} from './workspaceSettings/SupabaseProjectStep.svelte'
	import { supabaseResourceValue } from './workspaceSettings/supabaseProvisioning'
	import { useSupabaseOauth } from './workspaceSettings/supabaseOauth.svelte'
	import { sendUserToast } from '$lib/toast'

	interface Props {
		resourceType: string
		resourceTypeInfo: ResourceType | undefined
		args?: Record<string, any> | any
		linkedSecrets?: string[]
		isValid?: boolean
		linkedSecretCandidates?: string[] | undefined
		description?: string | undefined
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
		onSynced = undefined
	}: Props = $props()

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
		const regex =
			/postgres(?:ql)?:\/\/(?<user>[^:@]+)(?::(?<password>[^@]+))?@(?<host>[^:\/?]+)(?::(?<port>\d+))?\/(?<dbname>[^\?]+)?(?:\?.*sslmode=(?<sslmode>[^&]+))?/
		const match = connectionString.match(regex)
		if (match) {
			validConnectionString = true
			const { user, password, host, port, dbname, sslmode } = match.groups!
			rawCode = JSON.stringify(
				{
					...args,
					user,
					password: password || args?.password,
					host,
					port: (port ? Number(port) : undefined) || args?.port,
					dbname: dbname || args?.dbname,
					sslmode: sslmode || args?.sslmode
				},
				null,
				2
			)
			rawCodeEditor?.setCode(rawCode)
			close(null)
		} else {
			validConnectionString = false
		}
	}

	let rawCodeEditor: { setCode: (code: string) => void } | undefined = $state(undefined)
	let textFileContent: string | undefined = $state(undefined)

	let supabaseOpen = $state(false)
	let supaStep: ReturnType<typeof SupabaseProjectStep> | undefined = $state(undefined)
	let supaResult: SupabasePick | undefined = $state(undefined)

	// Authorizing is not something to present a dialog about first: the button goes straight
	// to the popup, and the dialog opens on the way back, already holding the projects.
	const supaOauth = useSupabaseOauth()
	let awaitingSupabaseAuth = $state(false)

	function connectSupabase() {
		if (supaOauth.authed) {
			supabaseOpen = true
			return
		}
		awaitingSupabaseAuth = true
		supaOauth.connect()
	}

	$effect(() => {
		if (awaitingSupabaseAuth && supaOauth.authed) {
			awaitingSupabaseAuth = false
			supabaseOpen = true
		}
	})

	// The resource is being edited here rather than created for us, so the project's password
	// goes straight into the form as a value. The user can link it to a secret variable with
	// the same affordance every other password field has.
	function applySupabasePick(pick: SupabasePick) {
		args = {
			...(args ?? {}),
			...supabaseResourceValue(pick.project, ''),
			password: pick.password
		}
		rawCode = JSON.stringify(args, null, 2)
		rawCodeEditor?.setCode(rawCode)
		supabaseOpen = false
		supaResult = undefined
		sendUserToast(`Filled in the connection for ${pick.project.name}`)
	}

	$effect(() => {
		if (supaResult) applySupabasePick(supaResult)
	})

	function parseTextFileContent() {
		args = {
			content: textFileContent
		}
	}
	$effect(() => {
		$workspaceStore && untrack(() => loadSchema())
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
			<Button
				unifiedSize="md"
				variant="default"
				startIcon={{ icon: SupabaseIcon }}
				loading={awaitingSupabaseAuth}
				on:click={connectSupabase}
			>
				Connect Supabase
			</Button>
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

<Modal2
	bind:isOpen={supabaseOpen}
	target="#content"
	title="Connect Supabase"
	contentClasses="flex flex-col"
	fixedWidth="md"
	fixedHeight="md"
>
	<div class="flex h-full flex-col gap-3">
		<div class="flex-1 flex flex-col gap-3 min-h-0">
			<SupabaseProjectStep
				bind:this={supaStep}
				bind:result={supaResult}
				defaultProjectName={`windmill-${$workspaceStore ?? 'workspace'}`}
				continueLabel="Use this project"
			/>
		</div>
		{#if supaStep}
			{@const action = supaStep.getAction()}
			<div class="flex justify-end pt-3">
				<Button
					size="sm"
					variant="accent"
					disabled={action.disabled}
					loading={action.busy}
					onClick={() => action.act?.()}
				>
					{action.label}
				</Button>
			</div>
		{/if}
	</div>
</Modal2>
