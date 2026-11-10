<script lang="ts">
	import { untrack } from 'svelte'
	import Select from '$lib/components/select/Select.svelte'
	import { AiService, type AiArtifact } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { apiErrorMessage, displayDate } from '$lib/utils'
	import { logFeatureUsage } from '$lib/utils/featureUsage'
	import {
		currentVersion,
		getArtifact,
		getArtifactVersion,
		listArtifactVersions,
		type ArtifactVersion,
		type PersistedArtifact
	} from './artifactsDB'
	import { showVersion } from './artifactLibrary'

	interface Props {
		/** The artifact's home workspace, where its server copy is. */
		workspace: string
		artifactId: string
		/** Called after the version people see changed. */
		onChange?: () => void
	}

	let { workspace, artifactId, onChange }: Props = $props()

	let server: AiArtifact | undefined = $state()
	/** The owner's copy, which holds every version; absent in a browser that did not write it. */
	let local: PersistedArtifact | undefined = $state()
	let versions: ArtifactVersion[] = $state([])
	let saving = $state(false)

	/** Read the server copy again, after its access list changed. */
	export async function reload() {
		const [row, artifact, stored] = await Promise.all([
			AiService.getAiArtifact({ workspace, artifactId }).catch(() => undefined),
			getArtifact(artifactId),
			listArtifactVersions(artifactId)
		])
		server = row
		local = artifact
		versions = stored
	}

	untrack(() => void reload())

	const shared = $derived(server !== undefined && Object.keys(server.extra_perms).length > 0)
	const selected = $derived(server?.pinned ? String(server.shown_version) : 'latest')
	const items = $derived.by(() => {
		const numbers = versions.map((v) => v.version)
		if (local) numbers.push(currentVersion(local))
		if (server?.pinned) numbers.push(server.shown_version)
		return [
			{ label: 'The latest version', value: 'latest' },
			...numbers
				.filter((n, i) => numbers.indexOf(n) === i)
				.sort((a, b) => b - a)
				.map((n) => ({ label: versionLabel(n), value: String(n) }))
		]
	})

	function versionLabel(n: number): string {
		const v = versions.find((x) => x.version === n)
		if (!v) return `v${n}`
		const note = v.note ?? (n === 1 ? 'Created' : 'Edited')
		return `v${n} · ${note} · ${displayDate(new Date(v.savedAt))}`
	}

	async function choose(value: string) {
		if (value === selected || !local || !server) return
		const pinned = value !== 'latest'
		const latest = currentVersion(local)
		const version = pinned ? Number(value) : latest
		const snapshot =
			version === latest
				? { name: local.name, content: local.content }
				: await getArtifactVersion(artifactId, version).catch(() => undefined)
		if (!snapshot) {
			sendUserToast(`Version ${version} is no longer in this browser`, true)
			return
		}
		saving = true
		try {
			const result = await showVersion(local, workspace, server.latest_version, {
				pinned,
				version,
				name: snapshot.name,
				content: snapshot.content
			})
			if (result.outcome === 'saved' && result.artifact) {
				server = result.artifact
				logFeatureUsage('ai_artifacts', 'pin', { key: pinned ? 'version' : 'latest' })
				onChange?.()
			} else if (result.outcome === 'stale') {
				sendUserToast(
					`Version ${result.latest_version} was written in another browser, and is not in this one yet`,
					true
				)
			} else {
				sendUserToast('This artifact is in Recently deleted', true)
			}
		} catch (err) {
			sendUserToast('Could not change the version people see', true, [], apiErrorMessage(err))
		} finally {
			saving = false
		}
	}
</script>

{#if shared}
	<section class="flex flex-col gap-2">
		<h3 class="text-sm font-semibold text-emphasis">People with access see</h3>
		<Select
			{items}
			bind:value={() => selected, (v) => v !== undefined && void choose(v)}
			disabled={!local || saving}
			size="sm"
		/>
		<span class="text-2xs text-secondary">
			{#if server?.is_owner === false}
				Only its owner chooses the version people see.
			{:else if !local}
				This artifact was written in another browser: choose the version from there.
			{:else if selected === 'latest'}
				Each new version reaches them as it is written.
			{:else}
				They stay on this version while new ones are written.
			{/if}
		</span>
	</section>
{/if}
