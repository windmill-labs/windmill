<script lang="ts">
	import { WorkspaceService, type Script, type WorkspaceDefaultScripts } from '$lib/gen'
	import { defaultScripts, workspaceStore } from '$lib/stores'
	import { flip } from 'svelte/animate'
	import Toggle from './Toggle.svelte'
	import { defaultScriptLanguages } from '$lib/scripts'
	import Alert from './common/alert/Alert.svelte'

	export let small = false
	$: langs = computeLangs($defaultScripts)

	function computeLangs(defaultScripts: WorkspaceDefaultScripts | undefined): Script['language'][] {
		const allLangs = Object.keys(defaultScriptLanguages) as Script['language'][]
		if (!defaultScripts || defaultScripts.order == undefined) return allLangs
		return defaultScripts.order
			?.concat(allLangs.filter((l) => !defaultScripts.order?.includes(l)))
			.filter((x) => x != 'nativets') as Script['language'][]
	}

	async function changePosition(i: number, up: boolean) {
		let norder = langs
		if (up) {
			;[norder[i], norder[i - 1]] = [norder[i - 1], norder[i]]
		} else {
			;[norder[i], norder[i + 1]] = [norder[i + 1], norder[i]]
		}
		defaultScripts.update((s) => ({ ...s, order: norder }))
		await WorkspaceService.editDefaultScripts({
			workspace: $workspaceStore!,
			requestBody: $defaultScripts
		})
	}
</script>

<Alert title="Global to workspace" type="info" class="mb-4" size={small ? 'xs' : 'sm'}>
	This setting is only available to admins and will affect all users in the workspace.
</Alert>
<div class="h-full w-full flex-col {small ? 'gap-0' : 'gap-2'} flex">
	{#each langs as lang, i (lang)}
		<div
			animate:flip={{ duration: 300 }}
			class="w-full p-2 rounded {small
				? ''
				: 'border border-secondary'} grid grid-cols-3 items-center"
		>
			<p class={small ? 'text-2xs font-medium justify-center' : 'text-sm'}>{lang}</p>
			<div>
				{#if i > 0}
					<button
						on:click={() => changePosition(i ?? 0, true)}
						class={small ? 'mr-2 text-secondary text-sm' : 'text-lg mr-2'}
						title="Move up"
					>
						&uparrow;</button
					>
				{/if}
				{#if i < langs.length - 1}
					<button
						on:click={() => changePosition(i ?? 0, false)}
						class={small ? 'mr-2 text-secondary text-sm' : 'text-lg mr-2'}
						title="Move down">&downarrow;</button
					>
				{/if}</div
			>
			<!-- <Toggle options={{ right: 'custom default' }} size="xs" /> -->
			<div class="flex justify-end">
				<Toggle
					options={{ right: 'hide' }}
					size="xs"
					color="red"
					checked={$defaultScripts?.hidden?.includes(lang)}
					on:change={(e) => {
						let toggled = e.detail
						if (toggled) {
							defaultScripts.update((s) => ({
								...(s ?? {}),
								hidden: [...(s?.hidden ?? []), lang]
							}))
						} else {
							defaultScripts.update((s) => ({
								...(s ?? {}),
								hidden: (s?.hidden ?? []).filter((h) => h != lang)
							}))
						}
					}}
				/>
			</div>
		</div>
	{/each}
</div>
