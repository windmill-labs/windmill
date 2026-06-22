<script lang="ts">
	import { onMount } from 'svelte'
	import YAML from 'yaml'
	import Button from '../common/button/Button.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import SettingCard from '../instanceSettings/SettingCard.svelte'
	import Label from '../Label.svelte'
	import autosize from '$lib/autosize'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { WorkspaceService } from '$lib/gen'
	import { FolderUp, Plus, Trash2 } from 'lucide-svelte'

	type SkillListItem = { name: string; description: string }
	type SkillUpload = { name: string; description: string; instructions: string }

	// `<root>/<skill>/SKILL.md` is 3 path segments; SKILL.md files nested deeper
	// are likely vendored/incidental and are skipped so importing a parent dir
	// doesn't sweep in unrelated skills.
	const MAX_SKILL_DEPTH = 3
	const MAX_SKILLS_PER_IMPORT = 50
	const MAX_SKILLS_PER_WORKSPACE = 100
	// `name` + `description` mirror the Claude SKILL.md spec (counted in
	// characters); the body is a byte-bounded payload. Keep these in sync with
	// backend `validate_skill`.
	const MAX_SKILL_NAME_LENGTH = 64
	const MAX_SKILL_DESCRIPTION_LENGTH = 1_024
	const MAX_SKILL_INSTRUCTIONS_LENGTH = 64 * 1024
	const SKILL_NAME_PATTERN = /^[a-z0-9-]+$/
	const textEncoder = new TextEncoder()
	const SAMPLE_SKILL_PLACEHOLDER =
		'---\nname: my-skill\ndescription: what this skill helps with\n---\n\n# My skill\n\nInstructions for the assistant…'

	let skills: SkillListItem[] = $state([])
	let uploading: boolean = $state(false)
	let pasteContent: string = $state('')
	let dirInput: HTMLInputElement | undefined = $state(undefined)
	let toDelete: string | undefined = $state(undefined)
	let pendingImport: SkillUpload[] | undefined = $state(undefined)
	let pendingSkipped: string[] = $state([])
	let listRequestId = 0

	let pendingNamesPreview = $derived.by(() => {
		const p = pendingImport ?? []
		const shown = p
			.slice(0, 12)
			.map((s) => s.name)
			.join(', ')
		return p.length > 12 ? `${shown}, … (+${p.length - 12} more)` : shown
	})

	async function loadList(workspace: string | undefined) {
		const requestId = ++listRequestId
		if (!workspace) {
			skills = []
			return
		}
		try {
			const loaded = await WorkspaceService.listAiSkills({ workspace })
			if (requestId === listRequestId && workspace === $workspaceStore) {
				skills = loaded
			}
		} catch (e) {
			if (requestId === listRequestId && workspace === $workspaceStore) {
				sendUserToast(`Failed to load skills: ${e}`, true)
			}
		}
	}

	/** Split a SKILL.md into its frontmatter `name`/`description` and the markdown body. */
	function parseSkillMd(raw: string): {
		name: string | undefined
		description: string | undefined
		instructions: string
	} {
		const text = raw.replace(/^﻿/, '')
		const fm = /^---\s*\r?\n([\s\S]*?)\r?\n---\s*\r?\n?/.exec(text)
		if (!fm) {
			return { name: undefined, description: undefined, instructions: text.trim() }
		}
		let name: string | undefined
		let description: string | undefined
		try {
			const data = YAML.parse(fm[1]) ?? {}
			if (typeof data?.name === 'string') name = data.name.trim()
			if (typeof data?.description === 'string') description = data.description.trim()
		} catch {
			// Malformed frontmatter — fall through so the skill is reported as
			// invalid rather than silently dropped.
		}
		return { name, description, instructions: text.slice(fm[0].length).trim() }
	}

	function validateParsedSkill(skill: SkillUpload): string | undefined {
		if ([...skill.name].length > MAX_SKILL_NAME_LENGTH) {
			return `name is longer than ${MAX_SKILL_NAME_LENGTH} characters`
		}
		if (!SKILL_NAME_PATTERN.test(skill.name)) {
			return `name ${JSON.stringify(skill.name)} must only contain lowercase letters, digits or '-'`
		}
		if ([...skill.description].length > MAX_SKILL_DESCRIPTION_LENGTH) {
			return `description is longer than ${MAX_SKILL_DESCRIPTION_LENGTH} characters`
		}
		if (textEncoder.encode(skill.instructions).byteLength > MAX_SKILL_INSTRUCTIONS_LENGTH) {
			return `body is longer than ${MAX_SKILL_INSTRUCTIONS_LENGTH} bytes`
		}
	}

	/**
	 * Turn a map of `relativePath -> content` (from an imported folder) into skills.
	 * A skill is any `SKILL.md`; its id is the name of the folder holding it.
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
				const parsed = { name, description, instructions }
				const validationError = validateParsedSkill(parsed)
				if (validationError) {
					skipped.push(`${name} (${validationError})`)
				} else {
					collected.push(parsed)
				}
			}
		}
		return { skills: collected, skipped }
	}

	async function uploadSkills(parsed: SkillUpload[], skipped: string[] = []) {
		const workspace = $workspaceStore
		if (!workspace || parsed.length === 0) {
			sendUserToast(
				`No valid skill found.${skipped.length ? ` Skipped: ${skipped.join(', ')}` : ''}`,
				true
			)
			return false
		}
		if (parsed.length > MAX_SKILLS_PER_IMPORT) {
			sendUserToast(`Cannot add more than ${MAX_SKILLS_PER_IMPORT} skills at a time.`, true)
			return false
		}
		// Uploads upsert, so only names not already stored count toward the cap.
		const existingNames = new Set(skills.map((s) => s.name))
		const newCount = parsed.filter((s) => !existingNames.has(s.name)).length
		if (skills.length + newCount > MAX_SKILLS_PER_WORKSPACE) {
			sendUserToast(`This workspace can store at most ${MAX_SKILLS_PER_WORKSPACE} skills.`, true)
			return false
		}
		uploading = true
		try {
			await WorkspaceService.uploadAiSkills({
				workspace,
				requestBody: { skills: parsed }
			})
			let message = `Added ${parsed.length} skill(s)`
			if (skipped.length) message += `; skipped ${skipped.length}: ${skipped.join(', ')}`
			sendUserToast(message)
			await loadList(workspace)
			return true
		} catch (e) {
			sendUserToast(`Failed to add skills: ${e}`, true)
			return false
		} finally {
			uploading = false
		}
	}

	async function addPastedSkill() {
		const { name, description, instructions } = parseSkillMd(pasteContent)
		if (!name) {
			sendUserToast('The pasted SKILL.md needs a `name` in its frontmatter.', true)
			return
		}
		if (!description) {
			sendUserToast('The pasted SKILL.md needs a `description` in its frontmatter.', true)
			return
		}
		if (!instructions) {
			sendUserToast('The pasted SKILL.md has an empty body.', true)
			return
		}
		const parsed = { name, description, instructions }
		const validationError = validateParsedSkill(parsed)
		if (validationError) {
			sendUserToast(`The pasted SKILL.md ${validationError}.`, true)
			return
		}
		if (await uploadSkills([parsed])) {
			pasteContent = ''
		}
	}

	async function onDirSelected(event: Event) {
		const target = event.target as HTMLInputElement
		const files = Array.from(target.files ?? [])
		// Reset early so re-selecting the same folder re-fires `change`.
		if (dirInput) dirInput.value = ''

		// Pick SKILL.md files within the depth limit BEFORE reading any content,
		// so a huge tree never gets read in full.
		const skipped: string[] = []
		const eligible: File[] = []
		for (const f of files) {
			const path = f.webkitRelativePath || f.name
			const segments = path.split('/')
			if (segments[segments.length - 1]?.toLowerCase() !== 'skill.md') continue
			if (segments.length > MAX_SKILL_DEPTH) {
				skipped.push(`${path} (nested deeper than ${MAX_SKILL_DEPTH} folder levels)`)
				continue
			}
			eligible.push(f)
		}

		if (eligible.length === 0) {
			sendUserToast(
				`No SKILL.md found within ${MAX_SKILL_DEPTH} folder levels.${
					skipped.length ? ` Skipped ${skipped.length} deeper file(s).` : ''
				}`,
				true
			)
			return
		}
		if (eligible.length > MAX_SKILLS_PER_IMPORT) {
			sendUserToast(
				`Found ${eligible.length} skills in this folder; imports are limited to ${MAX_SKILLS_PER_IMPORT} at a time.`,
				true
			)
			return
		}

		const map: Record<string, string> = {}
		for (const f of eligible) {
			map[f.webkitRelativePath || f.name] = await f.text()
		}
		const { skills: parsed, skipped: parseSkipped } = collectSkills(map)
		const allSkipped = [...skipped, ...parseSkipped]
		if (parsed.length === 0) {
			sendUserToast(
				`No valid skill found.${allSkipped.length ? ` Skipped: ${allSkipped.join(', ')}` : ''}`,
				true
			)
			return
		}
		// Confirm before writing — the import can pull in several skills at once.
		pendingSkipped = allSkipped
		pendingImport = parsed
	}

	async function deleteSkill(name: string) {
		const workspace = $workspaceStore
		if (!workspace) return
		try {
			await WorkspaceService.deleteAiSkill({ workspace, name })
			sendUserToast(`Deleted skill ${name}`)
			await loadList(workspace)
		} catch (e) {
			sendUserToast(`Failed to delete skill: ${e}`, true)
		}
	}

	onMount(() => {
		return workspaceStore.subscribe((workspace) => {
			toDelete = undefined
			pendingImport = undefined
			pendingSkipped = []
			void loadList(workspace)
		})
	})
</script>

<SettingCard
	label="Custom skills"
	description="Add your own skills to the AI Chat. The expected format is the same as Claude or Codex."
>
	<div class="flex flex-col gap-3 pt-1">
		<Label label="Paste a SKILL.md file">
			<textarea
				bind:value={pasteContent}
				placeholder={SAMPLE_SKILL_PLACEHOLDER}
				class="w-full min-h-24 p-2 border border-gray-200 dark:border-gray-700 rounded-md bg-surface text-primary font-mono text-xs resize-y"
				rows="5"
				use:autosize
			></textarea>
			<div class="flex justify-end mt-2">
				<Button
					onclick={addPastedSkill}
					variant="default"
					unifiedSize="sm"
					startIcon={{ icon: Plus }}
					disabled={!pasteContent.trim() || uploading}
				>
					Add skill
				</Button>
			</div>
		</Label>

		<Label label="Import a folder of skills">
			<div class="flex mt-1">
				<Button
					onclick={() => dirInput?.click()}
					variant="default"
					unifiedSize="sm"
					startIcon={{ icon: FolderUp }}
					disabled={uploading}
				>
					{uploading ? 'Importing…' : 'Import folder'}
				</Button>
			</div>
			<input
				bind:this={dirInput}
				type="file"
				style="display: none;"
				onchange={onDirSelected}
				{...{ webkitdirectory: true, directory: true }}
			/>
		</Label>

		{#if skills.length > 0}
			<div class="rounded-md border divide-y">
				{#each skills as skill (skill.name)}
					<div class="flex items-center justify-between gap-4 px-3 py-2">
						<div class="min-w-0">
							<div class="text-xs font-semibold font-mono truncate">{skill.name}</div>
							<div class="text-2xs text-secondary truncate">{skill.description}</div>
						</div>
						<Button
							onclick={() => (toDelete = skill.name)}
							variant="default"
							color="red"
							unifiedSize="sm"
							startIcon={{ icon: Trash2 }}
							iconOnly
						/>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</SettingCard>

<ConfirmationModal
	open={pendingImport !== undefined}
	title="Import skills"
	confirmationText="Import"
	onConfirmed={async () => {
		const toImport = pendingImport
		const skipped = pendingSkipped
		pendingImport = undefined
		pendingSkipped = []
		if (toImport) await uploadSkills(toImport, skipped)
	}}
	onCanceled={() => {
		pendingImport = undefined
		pendingSkipped = []
	}}
>
	<span>
		Add {pendingImport?.length} skill(s) to the AI chat?
		<span class="font-mono text-xs">{pendingNamesPreview}</span>
		{#if pendingSkipped.length}
			<br /><span class="text-xs text-secondary"
				>{pendingSkipped.length} file(s) will be skipped.</span
			>
		{/if}
	</span>
</ConfirmationModal>

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
