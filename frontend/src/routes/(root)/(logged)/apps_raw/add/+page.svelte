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
	import { writable } from 'svelte/store'

	let nodraft = $page.url.searchParams.get('nodraft')
	const templatePath = $page.url.searchParams.get('template')
	const templateId = $page.url.searchParams.get('template_id')

	const importRaw = $importStore
	if ($importStore) {
		$importStore = undefined
	}

	const state = nodraft ? undefined : localStorage.getItem('app')

	let summary = ''
	let files: Record<string, { code: string }> = {
		'/index.tsx': {
			code: `import React from 'react';
import { createRoot } from 'react-dom/client';

const App = () => {
  return <div style={{ width: "100%" }}><h1>Hello, Wooorldd!</h1>
    <div style={{ width: "100%", height: "100%", background: "red" }}>BAR</div></div>;
};

const root = createRoot(document.getElementById('root')!);
root.render(<App />);`
		},
		'/index.css': {
			code: `
body {
	background: blue;
}`
		},
		'/package.json': {
			code: `{
	"dependencies": {
		"react": "18.3.1",
		"react-dom": "18.3.1"
	}
}`
		},
		'/policy.json': {
			code: 'foo'
		}
	}
	let runnables = writable({})

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

	loadApp()

	function extractValue(value: any) {
		files = value.files
		runnables.set(value.runnables)
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
		} else if (templatePath) {
			const template = await AppService.getAppByPath({
				workspace: $workspaceStore!,
				path: templatePath
			})
			extractValue(template.value)

			sendUserToast('App loaded from template')
			goto('?', { replaceState: true })
		} else if (templateId) {
			const template = await AppService.getAppByVersion({
				workspace: $workspaceStore!,
				id: parseInt(templateId)
			})
			extractValue(template.value)
			sendUserToast('App loaded from template')
			goto('?', { replaceState: true })
		} else if (!templatePath && state) {
			sendUserToast('App restored from browser stored autosave', false, [
				{
					label: 'Start from blank',
					callback: () => {
						files = {}
						runnables.set({})
					}
				}
			])
			extractValue(decodeState(state))
		}
	}
</script>

<!-- <div class="h-screen">
		{#key value}
			<AppEditor
				on:savedNewAppPath={(event) => {
					goto(`/apps/edit/${event.detail}`)
				}}
				{summary}
				app={value}
				path={''}
				{policy}
				fromHub={hubId != null}
				newApp={true}
				replaceStateFn={(path) => replaceState(path, $page.state)}
				gotoFn={(path, opt) => goto(path, opt)}
			/>
		{/key}
	</div> -->
<RawAppEditor
	{files}
	{runnables}
	{policy}
	path={''}
	{summary}
	newApp
	appPath={`u/${$userStore?.username}/raw_app`}
/>
