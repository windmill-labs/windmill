<script module lang="ts">
	import type { GetSettingsResponse } from '$lib/gen'
	import { randomUUID } from '$lib/utils/uuid'

	export type McpToolSetRow = { id: string; name: string; scope: string; instructions: string }
	export type McpToolSetsType = { sets: McpToolSetRow[] }

	type StoredSet = { scripts?: string[]; flows?: string[]; instructions?: string }

	export function convertMcpToolSetsFromBackend(
		stored: GetSettingsResponse['mcp_tool_sets']
	): McpToolSetsType {
		return {
			sets: Object.entries((stored ?? {}) as Record<string, StoredSet>).map(([name, set]) => ({
				id: randomUUID(),
				name,
				scope: [
					set.scripts?.length ? `mcp:scripts:${set.scripts.join(',')}` : '',
					set.flows?.length ? `mcp:flows:${set.flows.join(',')}` : ''
				]
					.filter(Boolean)
					.join(' '),
				instructions: set.instructions ?? ''
			}))
		}
	}

	/** The saved sets with new row ids, so each row remounts: its scope selector reads the
	 *  selection once, and would otherwise keep the discarded one. */
	export function discardMcpToolSets(saved: McpToolSetsType): McpToolSetsType {
		return { sets: saved.sets.map((s) => ({ ...s, id: randomUUID() })) }
	}

	export function convertMcpToolSetsToBackend(
		settings: McpToolSetsType
	): Record<string, StoredSet> {
		// Null-prototype, so a set named `constructor` is a key like any other.
		const out: Record<string, StoredSet> = Object.create(null)
		for (const row of settings.sets) {
			const name = row.name.trim()
			if (!name) throw 'Every tool set needs a name'
			if (name in out) throw `Two tool sets are named ${name}`
			const scripts: string[] = []
			const flows: string[] = []
			for (const scope of row.scope.split(' ').filter(Boolean)) {
				if (scope.startsWith('mcp:scripts:')) {
					scripts.push(...scope.slice('mcp:scripts:'.length).split(',').filter(Boolean))
				} else if (scope.startsWith('mcp:flows:')) {
					flows.push(...scope.slice('mcp:flows:'.length).split(',').filter(Boolean))
				} else if (scope === 'mcp:favorites') {
					throw `Favorites are each member's own: pick folders, scripts or flows for ${name}`
				}
			}
			if (scripts.length === 0 && flows.length === 0) throw `${name} selects no scripts or flows`
			out[name] = { scripts, flows, instructions: row.instructions.trim() || undefined }
		}
		return out
	}
</script>

<script lang="ts">
	import { Plus } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import CloseButton from '../common/CloseButton.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import McpScopeSelector from '../mcp/McpScopeSelector.svelte'
	import ClipboardPanel from '../details/ClipboardPanel.svelte'
	import SettingsFooter from './SettingsFooter.svelte'
	import SettingsPageHeader from '../settings/SettingsPageHeader.svelte'
	import { WorkspaceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { apiBaseUrl } from '$lib/apiBaseUrl.svelte'
	import { clone } from '$lib/utils'
	import { logFeatureUsage } from '$lib/utils/featureUsage'

	let {
		mcpToolSets = $bindable(),
		mcpToolSetsSaved = $bindable(),
		onDiscard = undefined
	}: {
		mcpToolSets: McpToolSetsType
		mcpToolSetsSaved: McpToolSetsType
		onDiscard?: () => void
	} = $props()

	const strip = (s: McpToolSetsType) =>
		s.sets.map(({ name, scope, instructions }) => ({ name, scope, instructions }))
	let hasUnsavedChanges = $derived(
		JSON.stringify(strip(mcpToolSets)) !== JSON.stringify(strip(mcpToolSetsSaved))
	)

	function setUrl(name: string) {
		return `${apiBaseUrl('mcp')}/api/mcp/w/${$workspaceStore}/s/${name.trim() || '<name>'}/mcp`
	}

	async function onSave() {
		try {
			const mcp_tool_sets = convertMcpToolSetsToBackend(mcpToolSets)
			const savedNames = new Set(mcpToolSetsSaved.sets.map((s) => s.name.trim()))
			await WorkspaceService.editMcpToolSets({
				workspace: $workspaceStore!,
				requestBody: { mcp_tool_sets }
			})
			if (Object.keys(mcp_tool_sets).some((name) => !savedNames.has(name))) {
				logFeatureUsage('mcp_tool_set', 'created')
			}
			mcpToolSetsSaved = clone(mcpToolSets)
			sendUserToast('MCP tool sets saved')
		} catch (e) {
			sendUserToast(e, true)
			throw e
		}
	}
</script>

<SettingsPageHeader
	title="MCP tool sets"
	description="A tool set is a selection of scripts and flows served at its own MCP URL. Give the URL to the members who should use it: they sign in to Windmill from their AI client and get each script and flow of the set they can run, as a tool. A set never grants access: a member gets only what their own permissions already allow."
/>

<div class="flex flex-col gap-4">
	{#each mcpToolSets.sets as set, i (set.id)}
		<div class="flex flex-col gap-3 rounded-md border p-4">
			<div class="flex items-start gap-2">
				<div class="flex flex-col gap-1 grow">
					<span class="text-xs font-semibold text-emphasis">Name</span>
					<TextInput
						bind:value={set.name}
						inputProps={{ placeholder: 'support' }}
						class="max-w-64"
					/>
				</div>
				<CloseButton
					small
					on:close={() => {
						mcpToolSets.sets = mcpToolSets.sets.filter((_, j) => j !== i)
					}}
				/>
			</div>
			<div class="flex flex-col gap-1">
				<span class="text-xs font-semibold text-emphasis">Scripts and flows</span>
				<McpScopeSelector
					workspaceId={$workspaceStore ?? ''}
					initialScope={set.scope}
					bind:scope={set.scope}
					toolSet
				/>
			</div>
			<div class="flex flex-col gap-1">
				<span class="text-xs font-semibold text-emphasis">Instructions</span>
				<span class="text-2xs text-secondary">
					Sent to the AI client along with the tools, for example which flow answers which question.
				</span>
				<TextInput
					underlyingInputEl="textarea"
					bind:value={set.instructions}
					inputProps={{
						rows: 3,
						placeholder: 'For questions about orders, use the order lookup flow.'
					}}
				/>
			</div>
			<div class="flex flex-col gap-1">
				<span class="text-xs font-semibold text-emphasis">URL</span>
				<ClipboardPanel content={setUrl(set.name)} />
			</div>
		</div>
	{/each}
	<div class="flex justify-center">
		<Button
			unifiedSize="md"
			variant="default"
			startIcon={{ icon: Plus }}
			onclick={() => {
				mcpToolSets.sets = [
					...mcpToolSets.sets,
					{ id: randomUUID(), name: '', scope: '', instructions: '' }
				]
			}}
		>
			New tool set
		</Button>
	</div>
</div>

<SettingsFooter
	class="mt-6 mb-16"
	inline
	{hasUnsavedChanges}
	{onSave}
	onDiscard={() => onDiscard?.()}
	saveLabel="Save tool sets"
/>
