<script lang="ts">
	import { DraftService, NewScript, Script, ScriptService, WorkerService } from '$lib/gen'
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { inferArgs } from '$lib/infer'
	import { initialCode } from '$lib/script_helpers'
	import { enterpriseLicense, userStore, workerTags, workspaceStore } from '$lib/stores'
	import { emptySchema, encodeState, getModifierKey, setQueryWithoutLoad } from '$lib/utils'
	import Path from './Path.svelte'
	import ScriptEditor from './ScriptEditor.svelte'
	import ScriptSchema from './ScriptSchema.svelte'
	import { dirtyStore } from './common/confirmationModal/dirtyStore'
	import { Alert, Badge, Button, Drawer, Kbd } from './common'
	import { faPlus, faSave } from '@fortawesome/free-solid-svg-icons'
	import LanguageIcon from './common/languageIcons/LanguageIcon.svelte'
	import type { SupportedLanguage } from '$lib/common'
	import Tooltip from './Tooltip.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import { Pen, X } from 'lucide-svelte'
	import autosize from 'svelte-autosize'
	import type Editor from './Editor.svelte'
	import {
		SCRIPT_SHOW_BASH,
		SCRIPT_SHOW_GO,
		SCRIPT_SHOW_PSQL,
		SCRIPT_CUSTOMISE_SHOW_KIND
	} from '$lib/consts'
	import UnsavedConfirmationModal from './common/confirmationModal/UnsavedConfirmationModal.svelte'
	import { sendUserToast } from '$lib/toast'
	import { isCloudHosted } from '$lib/cloud'
	import Awareness from './Awareness.svelte'
	import { Icon } from 'svelte-awesome'
	import { fade } from 'svelte/transition'

	export let script: NewScript
	export let initialPath: string = ''
	export let template: 'pgsql' | 'mysql' | 'script' | 'docker' = 'script'
	export let initialArgs: Record<string, any> = {}
	export let lockedLanguage = false
	export let topHash: string | undefined = undefined
	export let showMeta: boolean = false

	let metadataOpen =
		showMeta ||
		(initialPath == '' &&
			$page.url.searchParams.get('state') == undefined &&
			$page.url.searchParams.get('collab') == undefined)
	let advancedOpen = false

	let editor: Editor | undefined = undefined
	let scriptEditor: ScriptEditor | undefined = undefined

	loadWorkerGroups()

	async function loadWorkerGroups() {
		if (!$workerTags) {
			$workerTags = await WorkerService.getCustomTags()
		}
	}

	export function setCode(code: string): void {
		editor?.setCode(code)
	}

	const langs: [string, SupportedLanguage][] = [
		['Typescript', Script.language.DENO],
		['Python', Script.language.PYTHON3]
	]
	if (SCRIPT_SHOW_GO) {
		langs.push(['Go', Script.language.GO])
	}
	if (SCRIPT_SHOW_BASH) {
		langs.push(['Bash', Script.language.BASH])
	}
	const scriptKindOptions: {
		value: Script.kind
		title: string
		desc?: string
		documentationLink?: string
	}[] = [
		{
			value: Script.kind.SCRIPT,
			title: 'Action'
		},
		{
			value: Script.kind.TRIGGER,
			title: 'Trigger',
			desc: 'First module of flows to trigger them based on external changes. These kind of scripts are usually running on a schedule to periodically look for changes.',
			documentationLink: 'https://docs.windmill.dev/docs/flows/flow_trigger'
		},
		{
			value: Script.kind.APPROVAL,
			title: 'Approval',
			desc: 'Send notifications externally to ask for approval to continue a flow.',
			documentationLink: 'https://docs.windmill.dev/docs/flows/flow_approval'
		},
		{
			value: Script.kind.FAILURE,
			title: 'Error Handler',
			desc: 'Handle errors in flows after all retry attempts have been exhausted.',
			documentationLink: 'https://docs.windmill.dev/docs/flows/flow_error_handler'
		}
	]

	let pathError = ''
	let loadingSave = false
	let loadingDraft = false

	$: {
		;['collab', 'path'].forEach((x) => {
			if ($page.url.searchParams.get(x)) {
				$page.url.searchParams.delete(x)
			}
		})
		setQueryWithoutLoad($page.url, [{ key: 'state', value: encodeState(script) }])
	}

	if (script.content == '') {
		initContent(script.language, script.kind, template)
	}

	function initContent(
		language: 'deno' | 'python3' | 'go' | 'bash',
		kind: Script.kind | undefined,
		template: 'pgsql' | 'mysql' | 'script' | 'docker'
	) {
		scriptEditor?.disableCollaboration()
		script.content = initialCode(language, kind, template)
		scriptEditor?.inferSchema(script.content, language)
		if (script.content != editor?.getCode()) {
			setCode(script.content)
		}
	}

	async function editScript(): Promise<void> {
		loadingSave = true
		try {
			$dirtyStore = false
			localStorage.removeItem(script.path)

			script.schema = script.schema ?? emptySchema()
			try {
				await inferArgs(script.language, script.content, script.schema as any)
			} catch (error) {
				sendUserToast(
					`The main signature was not parsable. This script is considered to be without main function`
				)
			}

			const newHash = await ScriptService.createScript({
				workspace: $workspaceStore!,
				requestBody: {
					path: script.path,
					summary: script.summary,
					description: script.description ?? '',
					content: script.content,
					parent_hash: topHash,
					schema: script.schema,
					is_template: script.is_template,
					language: script.language,
					kind: script.kind,
					tag: script.tag,
					envs: script.envs
				}
			})
			history.replaceState(history.state, '', `/scripts/edit/${script.path}`)
			goto(`/scripts/get/${newHash}?workspace=${$workspaceStore}`)
		} catch (error) {
			sendUserToast(`Error while saving the script: ${error.body || error.message}`, true)
		}
		loadingSave = false
	}

	async function saveDraft(): Promise<void> {
		loadingDraft = true
		try {
			$dirtyStore = false
			localStorage.removeItem(script.path)

			script.schema = script.schema ?? emptySchema()
			try {
				await inferArgs(script.language, script.content, script.schema as any)
			} catch (error) {
				sendUserToast(
					`The main signature was not parsable. This script is considered to be without main function`
				)
			}

			if (initialPath == '') {
				await ScriptService.createScript({
					workspace: $workspaceStore!,
					requestBody: {
						path: script.path,
						summary: script.summary,
						description: script.description ?? '',
						content: script.content,
						schema: script.schema,
						is_template: script.is_template,
						language: script.language,
						kind: script.kind,
						tag: script.tag,
						draft_only: true,
						envs: script.envs
					}
				})
			}
			await DraftService.createDraft({
				workspace: $workspaceStore!,
				requestBody: {
					path: initialPath == '' ? script.path : initialPath,
					typ: 'script',
					value: script
				}
			})
			if (initialPath == '') {
				$dirtyStore = false
				goto(`/scripts/edit/${script.path}`)
			}
			sendUserToast('Saved as draft')
		} catch (error) {
			sendUserToast(
				`Error while saving the script as a draft: ${error.body || error.message}`,
				true
			)
		}
		loadingDraft = false
	}

	function computeDropdownItems() {
		let dropdownItems: { label: string; onClick: () => void }[] = [
			{
				label: 'Fork',
				onClick: () => {
					window.open(`/scripts/add?template=${initialPath}`)
				}
			},
			{
				label: 'Exit & See details',
				onClick: () => {
					goto(`/scripts/get/${initialPath}?workspace=${$workspaceStore}`)
				}
			}
		]

		return dropdownItems
	}

	function onKeyDown(event: KeyboardEvent) {
		switch (event.key) {
			case 's':
				if (event.ctrlKey || event.metaKey) {
					saveDraft()
					event.preventDefault()
				}
				break
		}
	}
</script>

<svelte:window on:keydown={onKeyDown} />

<UnsavedConfirmationModal />
{#if !$userStore?.operator}
	<Drawer placement="right" bind:open={metadataOpen} size="800px">
		<DrawerContent title="Metadata" on:close={() => (metadataOpen = false)}>
			<h2 class="border-b pb-1 mb-4">Path</h2>
			<Path
				bind:error={pathError}
				bind:path={script.path}
				{initialPath}
				namePlaceholder="script"
				kind="script"
			/>
			<h2 class="border-b pb-1 mt-10 mb-4">Summary</h2>

			<input
				type="text"
				bind:value={script.summary}
				placeholder="Short summary to be displayed when listed"
			/>
			<h2 class="border-b pb-1 mt-10 mb-4">Language</h2>

			{#if lockedLanguage}
				<div class="text-sm text-gray-600 italic mb-2">
					As a forked script, the language '{script.language}' cannot be modified.
				</div>
			{/if}
			<div class="flex flex-row gap-2 flex-wrap">
				{#each langs as [label, lang]}
					{@const isPicked = script.language == lang && template == 'script'}
					<Button
						size="sm"
						variant="border"
						color={isPicked ? 'blue' : 'dark'}
						btnClasses={isPicked ? '!border-2 !bg-blue-50/75' : 'm-[1px]'}
						on:click={() => {
							template = 'script'
							initContent(lang, script.kind, template)
							script.language = lang
						}}
						disabled={lockedLanguage}
					>
						<LanguageIcon {lang} />
						<span class="ml-2 py-2">{label}</span>
					</Button>
				{/each}
				{#if SCRIPT_SHOW_PSQL}
					<Button
						size="sm"
						variant="border"
						color={template == 'pgsql' ? 'blue' : 'dark'}
						btnClasses={template == 'pgsql' ? '!border-2 !bg-blue-50/75' : 'm-[1px]'}
						disabled={lockedLanguage}
						on:click={() => {
							template = 'pgsql'
							initContent(Script.language.DENO, script.kind, template)
							script.language = Script.language.DENO
						}}
					>
						<LanguageIcon lang="pgsql" /><span class="ml-2 py-2">PostgreSQL</span>
					</Button>
				{/if}
				<Button
					size="sm"
					variant="border"
					color={template == 'docker' ? 'blue' : 'dark'}
					btnClasses={template == 'docker' ? '!border-2 !bg-blue-50/75' : 'm-[1px]'}
					disabled={lockedLanguage}
					on:click={() => {
						if (isCloudHosted()) {
							sendUserToast(
								'You cannot use Docker scripts on the multi-tenant platform. Use a dedicated instance or self-host windmill instead.',
								true,
								[
									{
										label: 'Learn more',
										callback: () => {
											window.open('https://docs.windmill.dev/docs/advanced/docker', '_blank')
										}
									}
								]
							)
							return
						}
						template = 'docker'
						initContent(Script.language.BASH, script.kind, template)
						script.language = Script.language.BASH
					}}
				>
					<LanguageIcon lang="docker" /><span class="ml-2 py-2">Docker</span>
				</Button>
				<!-- <Button
					size="sm"
					variant="border"
					color={template == 'mysql' ? 'blue' : 'dark'}
					btnClasses={template == 'mysql' ? '!border-2 !bg-blue-50/75' : 'm-[1px]'}
					on:click={() => {
						script.language = Script.language.DENO
						template = 'mysql'
						initContent(script.language, script.kind, template)
					}}
				>
					<LanguageIcon lang="mysql" /><span class="ml-2 py-2">MySQL</span>
				</Button> -->
			</div>

			<h2 class="border-b pb-1 mt-10 mb-4">Description</h2>
			<textarea
				use:autosize
				bind:value={script.description}
				placeholder="Description displayed in the details page"
				class="text-sm"
			/>

			<h2 class="border-b pb-1 mt-10 mb-4"
				>Worker group tag <Tooltip
					documentationLink="https://docs.windmill.dev/docs/core_concepts/worker_groups"
					>The script will be executed on a worker configured to accept its worker group tag. For
					instance, you could setup an "highmem", or "gpu" worker group.</Tooltip
				></h2
			>
			<div class="max-w-sm">
				{#if $workerTags}
					{#if $workerTags?.length > 0}
						<select
							bind:value={script.tag}
							on:change={(e) => {
								if (script.tag == '') {
									script.tag = undefined
								}
							}}
						>
							{#if script.tag}
								<option value="">reset to default</option>
							{:else}
								<option value="" disabled selected>Worker Group</option>
							{/if}
							{#each $workerTags ?? [] as tag (tag)}
								<option value={tag}>{tag}</option>
							{/each}
						</select>
					{:else}
						<div class="text-sm text-gray-600 italic mb-2">
							No custom worker group defined on this instance
						</div>
					{/if}
				{/if}
			</div>
			{#if !isCloudHosted()}
				<h2 class="border-b pb-1 mt-10 mb-4"
					>Custom env variables<Tooltip
						>Additional static custom env variables to pass to the script.</Tooltip
					></h2
				>
				<div class="w-full">
					<span class="text-gray-600 text-xs pb-2">Format is: `{'<KEY>=<VALUE>'}`</span>
					{#if Array.isArray(script.envs ?? [])}
						{#each script.envs ?? [] as v, i}
							<div class="flex max-w-md mt-1 w-full items-center">
								<input type="text" bind:value={v} placeholder="<KEY>=<VALUE>" />
								<button
									transition:fade|local={{ duration: 50 }}
									class="rounded-full p-1 bg-white/60 duration-200 hover:bg-gray-200"
									aria-label="Clear"
									on:click={() => {
										script.envs && script.envs.splice(i, 1)
										script.envs = script.envs
									}}
								>
									<X size={14} />
								</button>
							</div>
						{/each}
						{#if script.envs && script.envs.length > 0}
							<div class="pt-2" />
							<Alert type="warning" title="Not passed in previews"
								>Static envs variables are not passed in preview but solely on deployed scripts.</Alert
							>
						{/if}
					{/if}
				</div>
			{/if}
			<div class="flex mt-2">
				<Button
					variant="border"
					color="dark"
					size="xs"
					btnClasses="mt-1"
					on:click={() => {
						if (script.envs == undefined || !Array.isArray(script.envs)) {
							script.envs = []
						}
						script.envs = script.envs.concat('')
					}}
				>
					<Icon data={faPlus} class="mr-2" />
					Add item
				</Button>
			</div>
		</DrawerContent>
	</Drawer>

	<div class="flex flex-col h-screen">
		<div class="flex flex-col w-full px-2 py-1 border-b shadow-sm">
			<div class="justify-between flex gap-8 w-full items-center px-2">
				<div class="min-w-64 w-full max-w-md">
					<input
						type="text"
						placeholder="Script summary"
						class="text-sm w-full font-semibold"
						bind:value={script.summary}
					/>
				</div>
				<div class="gap-4 flex">
					<div class="flex justify-start w-full">
						<div>
							<button
								on:click={async () => {
									metadataOpen = true
								}}
							>
								<Badge
									color="gray"
									class="center-center !bg-gray-300 !text-gray-600 !h-[28px]  !w-[70px] rounded-r-none"
								>
									<Pen size={12} class="mr-2" /> Path
								</Badge>
							</button>
						</div>
						<input
							type="text"
							readonly
							value={script.path}
							size={script.path?.length || 50}
							class="font-mono !text-xs !min-w-[96px] !max-w-[300px] !w-full !h-[28px] !my-0 !py-0 !border-l-0 !rounded-l-none"
							on:focus={({ currentTarget }) => {
								currentTarget.select()
							}}
						/>
					</div>
				</div>
				<div class="center-center">
					<button
						on:click={async () => {
							metadataOpen = true
						}}
					>
						<LanguageIcon lang={script.language} />
					</button>
				</div>
				{#if $enterpriseLicense && initialPath != ''}
					<Awareness />
				{/if}

				<div class="flex flex-row gap-x-4">
					<Button
						color="light"
						variant="border"
						size="xs"
						on:click={() => {
							metadataOpen = true
						}}
					>
						Metadata
					</Button>
					<Button
						color="dark"
						variant="border"
						size="xs"
						on:click={() => {
							advancedOpen = true
						}}
					>
						Customise
					</Button>
					<Button
						loading={loadingDraft}
						size="sm"
						startIcon={{ icon: faSave }}
						on:click={() => saveDraft()}
					>
						Save draft&nbsp;<Kbd small>{getModifierKey()}</Kbd>
						<Kbd small>S</Kbd>
					</Button>
					<Button
						loading={loadingSave}
						size="sm"
						startIcon={{ icon: faSave }}
						on:click={() => editScript()}
						dropdownItems={initialPath != '' ? computeDropdownItems : undefined}
					>
						Deploy
					</Button>
				</div>
			</div>
		</div>

		<Drawer bind:open={advancedOpen} size="800px">
			<DrawerContent title="Customise" on:close={() => (advancedOpen = false)}>
				{#if SCRIPT_CUSTOMISE_SHOW_KIND}
					<h2 class="border-b pb-1 mb-4">
						Script kind &nbsp;
						<Tooltip>
							Tag this script's purpose within flows such that it is available as the corresponding
							action.
						</Tooltip>
					</h2>
					<div class="flex flex-wrap gap-2">
						{#each scriptKindOptions as { value, title, desc, documentationLink }}
							{@const isPicked = script.kind === value}
							<Button
								size="sm"
								variant="border"
								color={isPicked ? 'blue' : 'dark'}
								btnClasses="font-medium {isPicked ? '!bg-blue-50/75' : ''}"
								on:click={() => {
									template = 'script'
									script.kind = value
									initContent(script.language, value, template)
								}}
							>
								{title}
								{#if desc}
									<Tooltip {documentationLink} class="mb-0.5 ml-1">
										{desc}
									</Tooltip>
								{/if}
							</Button>
						{/each}
					</div>
				{/if}
				<h2 class="border-b pb-1 mt-10 mb-4"
					>Arguments &nbsp;<Tooltip
						>The arguments are synced with the main signature but you may refine the parts that
						cannot be inferred from the type directly.</Tooltip
					></h2
				>
				<ScriptSchema bind:schema={script.schema} />
			</DrawerContent>
		</Drawer>
		<ScriptEditor
			collabMode
			edit={initialPath != ''}
			on:format={() => {
				saveDraft()
			}}
			bind:editor
			bind:this={scriptEditor}
			bind:schema={script.schema}
			path={script.path}
			bind:code={script.content}
			lang={script.language}
			{initialArgs}
			kind={script.kind}
			tag={script.tag}
		/>
	</div>
{:else}
	Script Builder not available to operators
{/if}
