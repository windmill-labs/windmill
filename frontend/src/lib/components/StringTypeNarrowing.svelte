<script lang="ts">
	import RadioButton from './RadioButton.svelte'
	import ResourceTypePicker from './ResourceTypePicker.svelte'

	export let pattern: string | undefined
	export let enum_: string[] | undefined
	export let format: string | undefined
	export let contentEncoding: 'base64' | 'binary' | undefined

	let kind: 'none' | 'pattern' | 'enum' | 'resource' | 'format' | 'base64' = 'none'
	let patternStr: string = pattern ?? ''

	let resource: string | undefined

	const FORMATS = [
		'email',
		'hostname',
		'uri',
		'uuid',
		'ipv4'
		// 'time',
		// 'date',
		// 'duration',
		// 'ipv6',
		// 'jsonpointer'
	]

	$: format =
		kind == 'resource' ? (resource != undefined ? `resource-${resource}` : 'resource') : undefined
	$: pattern = patternStr == '' ? undefined : patternStr
	$: contentEncoding = kind == 'base64' ? 'base64' : undefined
</script>

<RadioButton
	label="Kind"
	small={true}
	options={[
		['None', 'none'],
		['File (base64)', 'base64'],
		['Enum', 'enum'],
		['Resource Path', 'resource'],
		['Format', 'format'],
		['Pattern', 'pattern']
	]}
	bind:value={kind}
/>

{#if kind == 'pattern'}
	<label for="input" class="mb-2 text-gray-700 text-xs"
		>Pattern (Regex)
		<div class="flex flex-row">
			<input
				id="input"
				type="text"
				placeholder="^(\\([0-9]{3}\\))?[0-9]{3}-[0-9]{4}$"
				bind:value={patternStr}
			/>
			<button
				class="default-button-secondary mx-2 mb-1"
				on:click={() => {
					patternStr = ''
				}}>clear</button
			>
		</div>
	</label>
{:else if kind == 'enum'}
	<label for="input" class="mb-2 text-gray-700 text-xs"
		>Enums
		{#each enum_ ?? [] as e}
			<div class="flex flex-row max-w-md">
				<input id="input" type="text" bind:value={e} />
				<button
					class="default-button mx-6"
					on:click={() => {
						enum_ = (enum_ ?? []).filter((el) => el != e)
						if (enum_.length == 0) {
							enum_ = undefined
						}
					}}>-</button
				>
			</div>
		{/each}
		<div class="flex flex-row my-1">
			<button
				class="default-button"
				on:click={() => {
					if (enum_ == undefined) {
						enum_ = []
					}
					enum_ = enum_.concat('')
				}}>+</button
			>
			<button
				class="default-button-secondary ml-2"
				on:click={() => {
					enum_ = undefined
				}}>clear</button
			>
		</div>
	</label>
{:else if kind == 'resource'}
	<div class="mt-1" />
	<ResourceTypePicker bind:value={resource} />
{:else if kind == 'format'}
	<select class="mt-1" bind:value={format}>
		<option value={undefined} />
		{#each FORMATS as f}
			<option value={f}>{f}</option>
		{/each}
	</select>
{/if}
