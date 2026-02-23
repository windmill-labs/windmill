<script lang="ts">
	import SimpleEditor from './SimpleEditor.svelte'
	import FileExplorer from './FileExplorer.svelte'

	interface Props {
		args: Record<string, any>
	}

	let { args = $bindable({}) }: Props = $props()

	// Internal files map uses /-prefixed keys (matching tree node paths)
	let files: Record<string, string> = $state(
		Object.fromEntries(
			Object.entries(args ?? {}).map(([k, v]) => ['/' + k, String(v ?? '')])
		)
	)

	// Initialize selection synchronously so SimpleEditor mounts with correct content
	const initialKeys = Object.keys(files)
	const initialFile = initialKeys.find((k) => !k.endsWith('/'))
	let selectedPath: string | undefined = $state(initialFile ?? (initialKeys.length > 0 ? '/' : '/'))
	let editContent: string = $state(initialFile ? (files[initialFile] ?? '') : '')

	// The selected file path (/-prefixed, not a folder)
	const selectedFileKey: string | undefined = $derived.by(() => {
		if (selectedPath != null && !selectedPath.endsWith('/')) {
			return selectedPath
		}
		return undefined
	})

	// Display key without leading /
	const selectedDisplayKey: string | undefined = $derived(
		selectedFileKey?.replace(/^\//, '')
	)

	function handleSelectPath(path: string) {
		selectedPath = path
		if (!path.endsWith('/') && path !== '') {
			editContent = files[path] ?? ''
		}
	}

	// Sync editContent → files → args reactively
	$effect(() => {
		const key = selectedFileKey
		const content = editContent
		if (key != null && key in files && files[key] !== content) {
			files = { ...files, [key]: content }
		}
	})

	// Sync files → args (strip / prefix, skip folder entries)
	$effect(() => {
		const newArgs: Record<string, any> = {}
		for (const [key, value] of Object.entries(files)) {
			if (!key.endsWith('/')) {
				newArgs[key.replace(/^\//, '')] = value
			}
		}
		args = newArgs
	})

	function inferLang(filePath: string): string {
		const ext = filePath.split('.').pop()?.toLowerCase()
		if (!ext) return 'plaintext'
		const langMap: Record<string, string> = {
			json: 'json',
			yaml: 'yaml',
			yml: 'yaml',
			toml: 'toml',
			ini: 'ini',
			xml: 'xml',
			html: 'html',
			css: 'css',
			js: 'javascript',
			ts: 'typescript',
			py: 'python',
			sh: 'shell',
			bash: 'shell',
			sql: 'sql',
			md: 'markdown',
			cfg: 'ini',
			conf: 'ini',
			j2: 'jinja',
			jinja: 'jinja'
		}
		return langMap[ext] ?? 'plaintext'
	}
</script>

<div class="flex border rounded-md overflow-hidden" style="min-height: 200px; max-height: 60vh;">
	<div class="w-56 shrink-0 border-r flex flex-col bg-surface-secondary">
		<FileExplorer
			bind:files
			{selectedPath}
			onSelectPath={handleSelectPath}
			showRoot
		/>
	</div>
	<div class="flex-1 min-w-0 overflow-y-auto">
		{#if selectedDisplayKey != null}
			<div class="px-3 py-1.5 border-b text-xs text-secondary bg-surface-secondary sticky top-0 z-10">
				{selectedDisplayKey}
			</div>
			{#key selectedFileKey}
				<SimpleEditor
					autoHeight
					lang={inferLang(selectedDisplayKey)}
					bind:code={editContent}
					fixedOverflowWidgets={false}
				/>
			{/key}
		{:else}
			<div class="flex items-center justify-center h-full text-xs text-secondary">
				Select a file or add a new one
			</div>
		{/if}
	</div>
</div>
