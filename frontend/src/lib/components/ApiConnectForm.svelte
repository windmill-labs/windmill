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
	import { base } from '$lib/base'
	import GitHubAppIntegration from './GitHubAppIntegration.svelte'
	import BedrockCredentialsCheck from './BedrockCredentialsCheck.svelte'
	import { isCloudHosted } from '$lib/cloud'

	interface Props {
		resourceType: string
		resourceTypeInfo: ResourceType | undefined
		args?: Record<string, any> | any
		linkedSecret?: string | undefined
		isValid?: boolean
		linkedSecretCandidates?: string[] | undefined
		description?: string | undefined
	}

	let {
		resourceType,
		resourceTypeInfo,
		args = $bindable({}),
		linkedSecret = $bindable(undefined),
		isValid = $bindable(true),
		linkedSecretCandidates = undefined,
		description = $bindable(undefined)
	}: Props = $props()

	let schema = $state(emptySchema())
	let notFound = $state(false)

	let supabaseWizard = $state(false)

	async function isSupabaseAvailable() {
		try {
			supabaseWizard =
				((await OauthService.listOauthConnects()) ?? {})['supabase_wizard'] != undefined
		} catch (error) {}
	}
	async function loadSchema() {
		if (!resourceTypeInfo) return
		rawCode = '{}'
		viewJsonSchema = false
		try {
			schema = resourceTypeInfo.schema as any
			schema.order = schema.order ?? Object.keys(schema.properties).sort()
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
		<TestConnection {resourceType} {args} />
		{#if resourceType == 'postgresql'}
			<Popover
				floatingConfig={{
					placement: 'bottom'
				}}
			>
				{#snippet trigger()}
					<Button spacingSize="sm" size="xs" variant="default" nonCaptureEvent>
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
			<a
				target="_blank"
				href="{base}/api/oauth/connect/supabase_wizard"
				class="border rounded-lg flex flex-row gap-2 items-center text-xs px-3 py-1.5 h-8 bg-[#F1F3F5] hover:bg-[#E6E8EB] dark:bg-[#1C1C1C] dark:hover:bg-black"
			>
				<SupabaseIcon height="16px" width="16px" />
				<div class="text-[#11181C] dark:text-[#EDEDED] font-semibold">Connect Supabase</div>
			</a>
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
{/if}
{#if notFound || viewJsonSchema}
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
	<h5 class="mt-1 inline-flex items-center gap-4">
		Fileset
	</h5>
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
		bind:linkedSecret
		isValid
		{schema}
		bind:args
	/>
{/if}
