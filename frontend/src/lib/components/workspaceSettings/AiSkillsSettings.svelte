<script lang="ts">
	import { onMount } from 'svelte'
	import JSZip from 'jszip'
	import YAML from 'yaml'
	import Button from '../common/button/Button.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { workspaceStore, userStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { WorkspaceService } from '$lib/gen'
	import { FolderUp, FileUp, RefreshCw, Trash2 } from 'lucide-svelte'

	type SkillListItem = { name: string; description: string }
	type SkillUpload = { name: string; description: string; instructions: string }

	let skills: SkillListItem[] = $state([])
	let loading: boolean = $state(false)
	let uploading: boolean = $state(false)
	let dirInput: HTMLInputElement | undefined = $state(undefined)
	let zipInput: HTMLInputElement | undefined = $state(undefined)
	let toDelete: string | undefined = $state(undefined)

	const isAdmin = $derived(!!$userStore?.is_admin)

	async function loadList() {
		if (!$workspaceStore) return
		loading = true
		try {
			skills = await WorkspaceService.listAiSkills({ workspace: $workspaceStore })
		} catch (e) {
			sendUserToast(`Failed to load skills: ${e}`, true)
		} finally {
			loading = false
		}
	}

	/** Split a SKILL.md into its frontmatter `description` and the markdown body. */
	function parseSkillMd(raw: string): { description: string | undefined; instructions: string } {
		const text = raw.replace(/^﻿/, '')
		const fm = /^---\s*\r?\n([\s\S]*?)\r?\n---\s*\r?\n?/.exec(text)
		if (!fm) {
			return { description: undefined, instructions: text.trim() }
		}
		let description: string | undefined
		try {
			const data = YAML.parse(fm[1]) ?? {}
			if (data && typeof data.description === 'string') {
				description = data.description.trim()
			}
		} catch {
			// Malformed frontmatter — fall through with no description so the skill
			// is reported as skipped rather than silently dropped.
		}
		return { description, instructions: text.slice(fm[0].length).trim() }
	}

	/**
	 * Turn a map of `relativePath -> content` (from a directory pick or a zip) into
	 * skills. A skill is any `SKILL.md`; its id is the name of the folder holding it.
	 */
	function collectSkills(files: Record<string, string>): {
		skills: SkillUpload[]
		skipped: string[]
	} {
		const collected: SkillUpload[] = []
		const skipped: string[] = []
		for (const [path, content] of Object.entries(files)) {
			const segments = path.split('/')
			if (segments[segments.length - 1]?.toLowerCase() !== 'skill.md') continue
			const name = segments.length >= 2 ? segments[segments.length - 2] : ''
			const { description, instructions } = parseSkillMd(content)
			if (!name) {
				skipped.push(`${path} (SKILL.md must live in a named folder)`)
			} else if (!description) {
				skipped.push(`${name} (missing frontmatter description)`)
			} else if (!instructions) {
				skipped.push(`${name} (empty body)`)
			} else {
				collected.push({ name, description, instructions })
			}
		}
		return { skills: collected, skipped }
	}

	async function uploadFileMap(files: Record<string, string>) {
		if (!$workspaceStore) return
		const { skills: parsed, skipped } = collectSkills(files)
		if (parsed.length === 0) {
			sendUserToast(
				`No valid SKILL.md found.${skipped.length ? ` Skipped: ${skipped.join(', ')}` : ''}`,
				true
			)
			return
		}
		uploading = true
		try {
			await WorkspaceService.uploadAiSkills({
				workspace: $workspaceStore,
				requestBody: { skills: parsed }
			})
			let message = `Uploaded ${parsed.length} skill(s)`
			if (skipped.length) message += `; skipped ${skipped.length}: ${skipped.join(', ')}`
			sendUserToast(message)
			await loadList()
		} catch (e) {
			sendUserToast(`Failed to upload skills: ${e}`, true)
		} finally {
			uploading = false
		}
	}

	async function onDirSelected(event: Event) {
		const target = event.target as HTMLInputElement
		const files = Array.from(target.files ?? [])
		const skillFiles = files.filter(
			(f) => (f.webkitRelativePath || f.name).split('/').pop()?.toLowerCase() === 'skill.md'
		)
		const map: Record<string, string> = {}
		for (const f of skillFiles) {
			map[f.webkitRelativePath || f.name] = await f.text()
		}
		await uploadFileMap(map)
		if (dirInput) dirInput.value = ''
	}

	async function onZipSelected(event: Event) {
		const target = event.target as HTMLInputElement
		const file = target.files?.[0]
		if (!file) return
		uploading = true
		try {
			const zip = await JSZip.loadAsync(await file.arrayBuffer())
			const map: Record<string, string> = {}
			const tasks: Promise<void>[] = []
			zip.forEach((relativePath, entry) => {
				if (entry.dir) return
				if (relativePath.split('/').pop()?.toLowerCase() !== 'skill.md') return
				tasks.push(
					entry.async('string').then((content) => {
						map[relativePath] = content
					})
				)
			})
			await Promise.all(tasks)
			await uploadFileMap(map)
		} catch (e) {
			sendUserToast(`Failed to read zip: ${e}`, true)
		} finally {
			uploading = false
			if (zipInput) zipInput.value = ''
		}
	}

	async function deleteSkill(name: string) {
		if (!$workspaceStore) return
		try {
			await WorkspaceService.deleteAiSkill({ workspace: $workspaceStore, name })
			sendUserToast(`Deleted skill ${name}`)
			await loadList()
		} catch (e) {
			sendUserToast(`Failed to delete skill: ${e}`, true)
		}
	}

	onMount(() => {
		loadList()
	})
</script>

<div class="mb-6">
	<h2 class="text-lg font-semibold mb-1">AI chat skills</h2>
	<p class="text-sm text-secondary">
		Reusable instruction sets for the Windmill AI chat in global mode. Each skill is a folder
		containing a <code>SKILL.md</code> file with YAML frontmatter (a <code>description</code>) and a
		markdown body — the same format Claude and Codex use. The chat advertises each skill's name and
		description in its system prompt and loads the full instructions on demand via the
		<code>read_skill</code> tool.
	</p>
	<p class="text-sm text-secondary mt-2">
		Upload a directory of skill folders (or a zip of them). Re-uploading a skill with the same
		folder name overwrites it.
	</p>
</div>

<div class="flex items-center gap-2 mb-4">
	<Button
		size="xs"
		variant="default"
		startIcon={{ icon: RefreshCw }}
		on:click={loadList}
		disabled={loading}
	>
		Refresh
	</Button>
	{#if isAdmin}
		<Button
			size="xs"
			variant="default"
			startIcon={{ icon: FolderUp }}
			on:click={() => dirInput?.click()}
			disabled={uploading}
		>
			{uploading ? 'Uploading…' : 'Upload folder'}
		</Button>
		<Button
			size="xs"
			variant="default"
			startIcon={{ icon: FileUp }}
			on:click={() => zipInput?.click()}
			disabled={uploading}
		>
			Upload zip
		</Button>
		<input
			bind:this={dirInput}
			type="file"
			style="display: none;"
			onchange={onDirSelected}
			{...{ webkitdirectory: true, directory: true }}
		/>
		<input
			bind:this={zipInput}
			type="file"
			accept=".zip"
			style="display: none;"
			onchange={onZipSelected}
		/>
	{/if}
</div>

{#if skills.length === 0}
	<div class="rounded border border-dashed p-6 text-center text-sm text-secondary">
		{loading ? 'Loading…' : 'No skills yet. Upload a folder of SKILL.md skill directories to get started.'}
	</div>
{:else}
	<div class="rounded border divide-y">
		{#each skills as skill (skill.name)}
			<div class="flex items-center justify-between gap-4 px-3 py-2">
				<div class="min-w-0">
					<div class="text-sm font-semibold font-mono truncate">{skill.name}</div>
					<div class="text-xs text-secondary truncate">{skill.description}</div>
				</div>
				{#if isAdmin}
					<Button
						size="xs"
						variant="default"
						color="red"
						startIcon={{ icon: Trash2 }}
						iconOnly
						on:click={() => (toDelete = skill.name)}
					/>
				{/if}
			</div>
		{/each}
	</div>
{/if}

<ConfirmationModal
	open={toDelete !== undefined}
	title="Delete skill"
	confirmationText="Delete"
	onConfirmed={async () => {
		const name = toDelete
		toDelete = undefined
		if (name) await deleteSkill(name)
	}}
	onCanceled={() => (toDelete = undefined)}
>
	<span>
		Delete the skill <code>{toDelete}</code>? The AI chat will no longer be able to use it.
	</span>
</ConfirmationModal>
