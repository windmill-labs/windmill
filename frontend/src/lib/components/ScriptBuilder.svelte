<script lang="ts">
	import { Script, ScriptService } from '$lib/gen'
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { inferArgs } from '$lib/infer'
	import { initialCode } from '$lib/script_helpers'
	import { userStore, workspaceStore } from '$lib/stores'
	import { emptySchema, encodeState, sendUserToast, setQueryWithoutLoad } from '$lib/utils'
	import Path from './Path.svelte'
	import ScriptEditor from './ScriptEditor.svelte'
	import ScriptSchema from './ScriptSchema.svelte'
	import CenteredPage from './CenteredPage.svelte'
	import { dirtyStore } from './common/confirmationModal/dirtyStore'
	import { Button, ButtonPopup, ButtonPopupItem, Kbd } from './common'
	import { faPen, faSave } from '@fortawesome/free-solid-svg-icons'
	import Breadcrumb from './common/breadcrumb/Breadcrumb.svelte'
	import LanguageIcon from './common/languageIcons/LanguageIcon.svelte'
	import type { SupportedLanguage } from '$lib/common'
	import Tooltip from './Tooltip.svelte'
	import SettingSection from './SettingSection.svelte'

	export let script: Script
	export let initialPath: string = ''
	export let template: 'pgsql' | 'mysql' | 'script' = 'script'
	export let initialArgs: Record<string, any> = {}
	export let lockedLanguage = false

	const langs: [string, SupportedLanguage][] = [
		['Typescript', Script.language.DENO],
		['Python', Script.language.PYTHON3],
		['Go', Script.language.GO],
		['Bash', Script.language.BASH]
	]
	const scriptKindOptions: { value: Script.kind; title: string; desc?: string }[] = [
		{
			value: Script.kind.SCRIPT,
			title: 'Action'
		},
		{
			value: Script.kind.TRIGGER,
			title: 'Trigger',
			desc: 'First module of flows to trigger them based on external changes. These kind of scripts are usually running on a schedule to periodically look for changes.'
		},
		{
			value: Script.kind.APPROVAL,
			title: 'Approval',
			desc: 'Send notifications externally to ask for approval to continue a flow.'
		},
		{
			value: Script.kind.FAILURE,
			title: 'Error Handler',
			desc: 'Handle errors in flows after all retry attempts have been exhausted.'
		}
	]

	let pathError = ''
	let summaryC: HTMLInputElement | undefined = undefined
	let pathC: Path | undefined = undefined
	let loadingSave = false

	$: setQueryWithoutLoad($page.url, [{ key: 'state', value: encodeState(script) }])
	$: step = Number($page.url.searchParams.get('step')) || 1

	if (script.content == '') {
		initContent(script.language, script.kind, template)
	}

	function initContent(
		language: 'deno' | 'python3' | 'go' | 'bash',
		kind: Script.kind,
		template: 'pgsql' | 'mysql' | 'script'
	) {
		script.content = initialCode(language, kind, template)
	}

	async function editScript(leave: boolean): Promise<void> {
		loadingSave = true
		try {
			$dirtyStore = false
			localStorage.removeItem(script.path)

			script.schema = script.schema ?? emptySchema()
			try {
				await inferArgs(script.language, script.content, script.schema)
			} catch (error) {
				sendUserToast(
					`Impossible to infer the schema. Assuming this is a script without main function`,
					true
				)
			}

			const newHash = await ScriptService.createScript({
				workspace: $workspaceStore!,
				requestBody: {
					path: script.path,
					summary: script.summary,
					description: script.description ?? '',
					content: script.content,
					parent_hash: script.hash != '' ? script.hash : undefined,
					schema: script.schema,
					is_template: script.is_template,
					language: script.language,
					kind: script.kind
				}
			})
			if (leave) {
				history.replaceState(history.state, '', `/scripts/edit/${newHash}?step=2`)
				goto(`/scripts/get/${newHash}?workspace_id=${$workspaceStore}`)
			} else {
				await goto(`/scripts/edit/${newHash}?step=2`)
				script.hash = newHash
			}
		} catch (error) {
			sendUserToast(`Impossible to save the script: ${error.body || error.message}`, true)
		}
		loadingSave = false
	}

	async function changeStep(step: number) {
		if (step > 1) {
			script.schema = script.schema ?? emptySchema()
			try {
				await inferArgs(script.language, script.content, script.schema)
			} catch (error) {
				console.info(
					'Impossible to infer the schema. Assuming this is a script without main function'
				)
			}
		}
		goto(`?step=${step}`)
	}

	$: kind = script.kind as 'script' | 'trigger' | 'approval' | undefined

	function onKeyDown(event: KeyboardEvent) {
		if (event.key == 'Enter' && step == 1) {
			changeStep(2)
		}
	}
</script>

{#if !$userStore?.operator}
	<div class="flex flex-col h-screen">
		<!-- Nav between steps-->
		<div class="flex flex-col w-full px-2 py-1  border-b shadow-sm">
			<div
				class="justify-between flex flex-row w-full items-center overflow-x-auto scrollbar-hidden px-2"
			>
				<div class="flex flex-row py-1">
					<Breadcrumb
						items={['Metadata', 'Code', 'Advanced']}
						selectedIndex={step}
						on:select={(e) => changeStep(e.detail.index + 1)}
						disabled={pathError != ''}
					>
						<svelte:fragment slot="separator">/</svelte:fragment>
					</Breadcrumb>
				</div>

				<div class="gap-1 flex-row hidden md:flex shrink overflow-hidden">
					<Button
						btnClasses="hidden lg:inline-flex"
						startIcon={{ icon: faPen }}
						variant="contained"
						color="light"
						size="xs"
						on:click={async () => {
							await changeStep(1)
							setTimeout(() => pathC?.focus(), 100)
						}}
					>
						{script.path}
					</Button>

					<Button
						startIcon={{ icon: faPen }}
						variant="contained"
						color="light"
						size="xs"
						on:click={async () => {
							await changeStep(1)
							setTimeout(() => summaryC?.focus(), 100)
						}}
					>
						<div class="max-w-[10em] !truncate">
							{script.summary == '' || !script.summary ? 'No summary' : script.summary}
						</div>
					</Button>
				</div>
				<div class="flex flex-row gap-x-2">
					<Button
						variant="border"
						size="sm"
						btnClasses={step == 1 ? 'hidden sm:invisible' : ''}
						on:click={() => changeStep(step - 1)}
					>
						Back
					</Button>
					<Button
						size="sm"
						variant={step == 1 ? 'contained' : 'border'}
						btnClasses={step == 3 ? 'invisible' : 'inline-flex gap-2'}
						disabled={step === 1 && pathError !== ''}
						on:click={() => changeStep(step + 1)}
					>
						Next {#if step == 1}<Kbd>Enter</Kbd>{/if}
					</Button>
					<ButtonPopup
						loading={loadingSave}
						size="sm"
						variant={step == 1 ? 'border' : 'contained'}
						disabled={step === 1 && pathError !== ''}
						startIcon={{ icon: faSave }}
						on:click={() => editScript(false)}
					>
						<svelte:fragment slot="main">Save</svelte:fragment>
						<ButtonPopupItem on:click={() => editScript(true)}>Save and exit</ButtonPopupItem>
						{#if initialPath != ''}
							<ButtonPopupItem
								on:click={() => {
									window.open(`/scripts/add?template=${initialPath}`)
								}}>Fork</ButtonPopupItem
							>
						{/if}
					</ButtonPopup>
				</div>
			</div>
		</div>

		<!-- metadata -->
		{#if step === 1}
			<CenteredPage>
				<h2 class="border-b pb-1 mt-8 mb-2">Path</h2>
				<Path
					bind:this={pathC}
					bind:error={pathError}
					bind:path={script.path}
					{initialPath}
					on:enter={() => changeStep(2)}
					namePlaceholder="script"
					kind="script"
				/>
				<h2 class="border-b pb-1 mt-12 mb-4">Summary</h2>
				<input
					type="text"
					bind:this={summaryC}
					bind:value={script.summary}
					placeholder="Short summary to be displayed when listed"
				/>

				<h2 class="border-b pb-1 mt-12 mb-6">Language</h2>
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
								script.language = lang
								template = 'script'
								initContent(lang, script.kind, template)
							}}
							disabled={lockedLanguage}
						>
							<LanguageIcon {lang} />
							<span class="ml-2 py-2">{label}</span>
						</Button>
					{/each}
					<Button
						size="sm"
						variant="border"
						color={template == 'pgsql' ? 'blue' : 'dark'}
						btnClasses={template == 'pgsql' ? '!border-2 !bg-blue-50/75' : 'm-[1px]'}
						disabled={lockedLanguage}
						on:click={() => {
							script.language = Script.language.DENO
							template = 'pgsql'
							initContent(script.language, script.kind, template)
						}}
					>
						<LanguageIcon lang="pgsql" /><span class="ml-2 py-2">PostgreSQL</span>
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
				<SettingSection
					title="Script kind"
					element="h3"
					tooltip="Tag this script as having a specific purpose inside flows. If it won't be used in flows,
				you don't have to worry about this."
					accordion
				>
					<div class="flex flex-wrap gap-2">
						{#each scriptKindOptions as { value, title, desc }}
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
								disabled={lockedLanguage}
							>
								{title}
								{#if desc}
									<Tooltip class="mb-0.5 ml-1">
										{desc}
									</Tooltip>
								{/if}
							</Button>
						{/each}
					</div>
				</SettingSection>
			</CenteredPage>
		{:else if step === 2}
			<ScriptEditor
				bind:schema={script.schema}
				path={script.path}
				bind:code={script.content}
				lang={script.language}
				{initialArgs}
				{kind}
			/>
		{:else if step === 3}
			<CenteredPage>
				<ScriptSchema bind:description={script.description} bind:schema={script.schema} />
			</CenteredPage>
		{/if}
	</div>
{:else}
	Script Builder not available to operators
{/if}

<svelte:window on:keydown={onKeyDown} />
