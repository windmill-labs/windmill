<script lang="ts">
	import { importStore } from '$lib/components/apps/store'

	import { AppService, type Policy } from '$lib/gen'
	import { page } from '$app/stores'
	import { decodeState } from '$lib/utils'
	import { userStore, workspaceStore } from '$lib/stores'
	import { afterNavigate, replaceState } from '$app/navigation'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'

	import RawAppEditor from '$lib/components/raw_apps/RawAppEditor.svelte'
	import type { HiddenRunnable } from '$lib/components/apps/types'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import FileEditorIcon from '$lib/components/raw_apps/FileEditorIcon.svelte'
	import { react18Template, react19Template, svelte5Template, vueTemplate } from './templates'

	let nodraft = $page.url.searchParams.get('nodraft')
	const templatePath = $page.url.searchParams.get('template')
	const templateId = $page.url.searchParams.get('template_id')

	const importRaw = $importStore
	if ($importStore) {
		$importStore = undefined
	}

	const state = nodraft ? undefined : localStorage.getItem('rawapp')

	let summary = ''
	let files: Record<string, string> = react19Template
	afterNavigate(() => {
		if (nodraft) {
			let url = new URL($page.url.href)
			url.search = ''
			replaceState(url.toString(), $page.state)
		}
	})
	let policy: Policy = {
		on_behalf_of: $userStore?.username.includes('@')
			? $userStore?.username
			: `u/${$userStore?.username}`,
		on_behalf_of_email: $userStore?.email,
		execution_mode: 'publisher'
	}

	let runnables: Record<string, HiddenRunnable> = {
		a: {
			name: 'a',
			fields: {},
			recomputeIds: [],
			type: 'runnableByName',
			inlineScript: {
				content:
					'// import * as wmill from "windmill-client"\n\nexport async function main(x: string) {\n  return x\n}\n',
				language: 'bun',
				schema: {
					$schema: 'https://json-schema.org/draft/2020-12/schema',
					properties: {
						x: {
							default: null,
							description: '',
							originalType: 'string',
							type: 'string'
						}
					},
					required: ['x'],
					type: 'object'
				}
			}
		}
	}
	loadApp()

	function extractValue(value: any) {
		files = value.files
		runnables = value.runnables
	}
	async function loadApp() {
		if (importRaw) {
			sendUserToast('Loaded from YAML/JSON')
			if ('value' in importRaw) {
				summary = importRaw.summary
				extractValue(importRaw.value)

				policy = importRaw.policy
			} else {
				extractValue(importRaw)
			}
			console.log('importRaw', importRaw)
		} else if (templatePath) {
			const template = await AppService.getAppByPath({
				workspace: $workspaceStore!,
				path: templatePath
			})
			extractValue(template.value)
			console.log('App loaded from template')
			sendUserToast('App loaded from template path')
			goto('?', { replaceState: true })
		} else if (templateId) {
			const template = await AppService.getAppByVersion({
				workspace: $workspaceStore!,
				id: parseInt(templateId)
			})
			extractValue(template.value)
			console.log('App loaded from template id')
			sendUserToast('App loaded from template')
			goto('?', { replaceState: true })
		} else if (!templatePath && state) {
			console.log('App loaded from browser stored autosave')
			sendUserToast('App restored from browser stored autosave', false, [
				{
					label: 'Start from blank',
					callback: () => {
						files = {}
						runnables = {}
					}
				}
			])
			let decoded = decodeState(state)
			extractValue(decoded)
		}
	}

	const templates = [
		{
			name: 'React 19',
			icon: 'tsx',
			files: undefined,
			selected: true
		},
		{
			name: 'React 18',
			icon: 'tsx',
			files: react18Template
		},
		{
			name: 'Svelte 5',
			icon: 'svelte',
			files: svelte5Template
		},
		{
			name: 'Vue 3',
			icon: 'vue',
			files: vueTemplate
		}
	]
	let templatePicker = nodraft != null
	let hide = false
</script>

{#if templatePicker}
	<Modal kind="X" open title="Templates">
		<div class="flex flex-wrap gap-4 pb-4">
			{#each templates as t}
				<button
					on:click={() => {
						if (t.files) {
							hide = true

							files = t.files
							hide = false
						}
						templatePicker = false
					}}
					class="w-24 h-24 flex justify-between py-5 flex-col {t.selected
						? 'bg-surface-selected'
						: ''} hover:bg-surface-hover border rounded-lg"
				>
					<div class="w-full flex items-center justify-center">
						<FileEditorIcon file={'.' + t.icon} />
					</div>
					<div class="center-center w-full">{t.name}</div>
				</button>
			{/each}
		</div>
	</Modal>
{/if}
{#if !hide}
	<RawAppEditor
		on:savedNewAppPath={(event) => {
			goto(`/apps_raw/edit/${event.detail}`)
		}}
		initFiles={files}
		initRunnables={runnables}
		{policy}
		path={''}
		{summary}
		newApp
	/>
{/if}
