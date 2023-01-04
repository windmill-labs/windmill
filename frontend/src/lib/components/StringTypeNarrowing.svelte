<script lang="ts">
	import RadioButton from './RadioButton.svelte'
	import ResourceTypePicker from './ResourceTypePicker.svelte'
	import { Button } from './common'

	export let pattern: string | undefined
	export let enum_: string[] | undefined
	export let format: string | undefined
	export let contentEncoding: 'base64' | 'binary' | undefined

	let kind: 'none' | 'pattern' | 'enum' | 'resource' | 'format' | 'base64' = computeKind()
	let patternStr: string = pattern ?? ''

	let resource: string | undefined

	const FORMATS = [
		'email',
		'hostname',
		'uri',
		'uuid',
		'ipv4',
		'yaml',
		'sql'
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

	function add() {
		enum_ = enum_ ? enum_.concat('') : ['']
	}

	function remove(item: string) {
		enum_ = (enum_ || []).filter((el) => el !== item)
		if (enum_.length == 0) {
			enum_ = undefined
		}
	}

	function computeKind(): 'base64' | 'none' | 'pattern' | 'enum' | 'resource' | 'format' {
		if (enum_ != undefined) {
			return 'enum'
		}
		if (contentEncoding == 'base64') {
			return 'base64'
		}
		if (pattern != undefined) {
			return 'pattern'
		}
		if (format != undefined) {
			if (format.startsWith('resource')) {
				return 'resource'
			}
			return 'format'
		}
		return 'none'
	}
</script>

<RadioButton
	label="Kind"
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
<div class="my-2" />

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
			<Button
				variant="border"
				color="blue"
				size="sm"
				btnClasses="mx-2 mb-1"
				on:click={() => {
					patternStr = ''
				}}>clear</Button
			>
		</div>
	</label>
{:else if kind == 'enum'}
	<label for="input" class="mb-2 text-gray-700 text-xs">
		Enums
		<div class="flex flex-col gap-1">
			{#each enum_ || [] as e}
				<div class="flex flex-row max-w-md">
					<input id="input" type="text" bind:value={e} />
					<Button size="sm" btnClasses="ml-6" on:click={() => remove(e)}>-</Button>
				</div>
			{/each}
		</div>
		<div class="flex flex-row my-1">
			<Button size="sm" on:click={add}>+</Button>
			<Button variant="border" size="sm" btnClasses="ml-2" on:click={() => (enum_ = undefined)}>
				Clear
			</Button>
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
