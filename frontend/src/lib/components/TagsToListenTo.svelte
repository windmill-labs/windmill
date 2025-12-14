<script lang="ts">
	import { defaultTags, nativeTags } from './worker_group'
	import { safeSelectItems } from './select/utils.svelte'
	import MultiSelect from './select/MultiSelect.svelte'
	import { superadmin, devopsRole } from '$lib/stores'
	import { twMerge } from 'tailwind-merge'
	import { SettingService, WorkerService } from '$lib/gen'
	import { CUSTOM_TAGS_SETTING } from '$lib/consts'
	import { sendUserToast } from '$lib/toast'

	type Props = {
		worker_tags: string[]
		customTags: string[] | undefined
		disabled?: boolean
		class?: string
	}
	let {
		worker_tags = $bindable([]),
		customTags = $bindable([]),
		disabled: _disabled = $bindable(false),
		class: clazz = ''
	}: Props = $props()

	let disabled = $derived(_disabled || !($superadmin || $devopsRole))

	let multiSelect = $state<MultiSelect<{ label?: string; value: any }> | undefined>(undefined)

	const searchText = $derived(multiSelect?.getFilteredInputText())

	async function createCustomTag(tag: string) {
		const tagName = tag.trim().replaceAll(' ', '_')
		// optimistic update
		worker_tags = [...worker_tags, tagName]
		try {
			// Get current custom tags
			const currentCustomTags = await WorkerService.getCustomTags({
				showWorkspaceRestriction: Boolean($superadmin || $devopsRole)
			})

			// Check if tag already exists
			if (currentCustomTags?.includes(tagName)) {
				sendUserToast('Tag already exists', false)
				return
			}

			// Add new tag to the list
			await SettingService.setGlobal({
				key: CUSTOM_TAGS_SETTING,
				requestBody: { value: [...(currentCustomTags ?? []), tagName] }
			})

			// Update local state if customTags is bound
			if (customTags) {
				customTags = [...customTags, tagName]
			}

			sendUserToast('Custom tag created and added successfully')
		} catch (err) {
			// rollback optimistic update
			worker_tags = worker_tags.filter((t) => t !== tagName)
			sendUserToast(`Could not create custom tag: ${err}`, true)
		}
	}
</script>

<MultiSelect
	items={safeSelectItems([...(customTags ?? []), ...worker_tags, ...defaultTags, ...nativeTags])}
	bind:this={multiSelect}
	bind:value={() => worker_tags, (w) => (worker_tags = w.map((s) => s.replaceAll(' ', '_')))}
	{disabled}
	class={twMerge(disabled ? 'border-0' : '', clazz)}
	allowClear={!disabled}
	onCreateItem={createCustomTag}
	createText={searchText ? `Create custom tag: ${searchText}` : 'Create custom tag'}
/>
