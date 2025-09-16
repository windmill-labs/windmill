<script lang="ts">
	import { page } from '$app/state'
	import { goto } from '$app/navigation'
	import { Plug, SquareFunction } from 'lucide-svelte'

	const schema = {
		$schema: 'https://json-schema.org/draft/2020-12/schema',
		properties: {
			provider: {
				type: 'object',
				format: 'ai-provider',
				placeholder: 'Select an AI provider'
			},
			user_message: {
				type: 'string',
				default: 'Hello, how are you?'
			},
			system_prompt: {
				type: 'string',
				default: 'You are a helpful assistant.'
			},
			image: {
				type: 'object',
				description: 'Image to send to the AI agent (optional)',
				format: 'resource-s3_object'
			},
			max_completion_tokens: {
				type: 'number'
			},
			temperature: {
				type: 'number',
				description:
					'Controls randomness in text generation. Range: 0.0 (deterministic) to 2.0 (random).'
			},
			output_type: {
				type: 'string',
				description:
					'The type of output the AI agent will generate (text or image). Image output will ignore tools, and only works with OpenAI, Google AI and OpenRouter gemini-image-preview model.',
				enum: ['text', 'image'],
				default: 'text'
			},
			output_schema: {
				type: 'object',
				description:
					'JSON schema that the AI agent will follow for its response format (only used if output_type is text)',
				format: 'json-schema'
			}
		},
		required: ['provider', 'model', 'user_message'],
		type: 'object',
		order: [
			'provider',
			'model',
			'user_message',
			'system_prompt',
			'image',
			'max_completion_tokens',
			'temperature',
			'output_type',
			'output_schema'
		]
	}

	// Initialize version from URL parameter, fallback to 'minimal'
	let version = $derived(page.url.searchParams.get('version') ?? 'minimal')
</script>

<div class="mx-auto max-w-xl min-h-screen min-w-96 p-4 flex flex-col gap-4">
	<select bind:value={() => version, (v) => goto(`?version=${v}`)}>
		<option value="minimal">Minimal</option>
		<option value="border">Border</option>
		<option value="border_nord">Border Nord</option>
		<option value="minimalistic">Minimalistic</option>
	</select>

	{#if version == 'minimal'}
		<div class="flex flex-col gap-2 divide-y divide-gray-200">
			{#each Object.entries(schema.properties) as [key, value]}
				{@render input({ key, value })}
			{/each}
		</div>

		{#snippet input({ key, value })}
			<div class="flex flex-col gap-1 rounded-md p-4">
				<!-- Header  -->
				<div class="flex flex-row gap-2">
					<label for={key} class="text-sm font-semibold">{key}</label>
					<div class="text-sm text-gray-400 italic">{value.type}</div>
				</div>
				<!-- Input -->
				<input
					type="text"
					class="border border-gray-300 p-2 !bg-surface-secondary !rounded-sm !shadow-none !border-none"
					id={key}
					placeholder={value.description}
				/>
			</div>
		{/snippet}
	{:else if version == 'border'}
		<!--
	Accent : blue-500
	Neutral : gray-600
	Highlight: gray-800 / semibold
	Secondary: gray-400
	Background surface-secondary: gray-50/50
	Border: gray-200
	callForAction: gray-500
	-->
		<div class="flex flex-col gap-4 divide-gray-200 p-4">
			{#each Object.entries(schema.properties) as [key, value]}
				{@render input({ key, value })}
			{/each}
		</div>

		{#snippet input({ key, value })}
			<div
				class="flex flex-col gap-2 rounded-md p-4 bg-gray-50/50 outline outline-1 outline-gray-200 transition-all duration-300 group"
			>
				<!-- Header  -->
				<div class="flex flex-row gap-2 justify-between items-center">
					<div class="flex flex-row gap-2">
						<label for={key} class="text-sm font-semibold text-gray-800 whitespace-nowrap"
							>{key}{#if schema.required.includes(key)}
								<span class="text-sm text-red-500">&nbsp;*</span>
							{/if}
						</label>

						<div class="text-sm text-gray-400 italic">{value.type}</div>
					</div>

					{@render actions?.()}
				</div>
				<!-- Input -->
				<input
					type="text"
					class="!rounded-md !bg-gray-50 !shadow-none !border-gray-200 hover:!border-gray-200 !text-gray-600 !focus:outline-none !focus:ring-none"
					id={key}
					placeholder={value.placeholder}
					value={value.default}
				/>
				<!-- Description -->
				<div class="text-2xs text-gray-400 italic">{value.description}</div>
			</div>
		{/snippet}

		{#snippet actions()}
			<div class="flex flex-row gap-2 h-6 -my-2">
				<button
					class="group-hover:opacity-100 opacity-0 py-1 px-2 w-fit duration-200 hover:bg-surface-hover rounded-md border flex items-center justify-center h-full text-gray-500"
				>
					<Plug size={14} />
				</button>
				{@render toggleButton?.()}
			</div>
			{#snippet toggleButton()}
				<div
					class="flex flex-row w-fit border border-gray-200 rounded-md transition-opacity duration-200 group-hover:opacity-100 opacity-0 h-full"
				>
					<button
						class="py-1 px-2 w-fit hover:bg-surface-hover rounded-md h-full text-xs font-normal text-gray-500"
					>
						{'${}'}
					</button>
					<button class=" py-1 px-2 w-fit rounded-md h-full bg-gray-200 text-gray-800">
						<SquareFunction size={14} />
					</button>
				</div>
			{/snippet}
		{/snippet}
	{:else if version == 'border_nord'}
		<!--
	Accent : nord-0
	Text Neutral : nord-300
	Text Highlight: nord-0 / semibold
	Text Secondary: nord-300/50
	Background surface-secondary: nord-600/20
	Border: nord-500
	callForAction: gray-500
	-->
		<div class="flex flex-col gap-4 p-4">
			{#each Object.entries(schema.properties) as [key, value]}
				{@render input({ key, value })}
			{/each}
		</div>

		{#snippet input({ key, value })}
			<div
				class="flex flex-col gap-2 rounded-md p-4 bg-nord-600/20 outline outline-1 outline-nord-500 transition-all duration-300 group"
			>
				<!-- Header  -->
				<div class="flex flex-row gap-2 justify-between items-center">
					<div class="flex flex-row gap-2">
						<label for={key} class="text-sm font-semibold text-nord-0 whitespace-nowrap"
							>{key}{#if schema.required.includes(key)}
								<span class="text-sm text-red-500">&nbsp;*</span>
							{/if}
						</label>

						<div class="text-sm text-nord-300/50 italic">{value.type}</div>
					</div>

					{@render actions?.()}
				</div>
				<!-- Input -->
				<input
					type="text"
					class="!rounded-md !bg-gray-50 !shadow-none !border-nord-500 hover:!border-nord-400 !text-nord-300 !focus:outline-none !focus:ring-none"
					id={key}
					placeholder={value.placeholder}
					value={value.default}
				/>
				<!-- Description -->
				<div class="text-2xs text-gray-400 italic">{value.description}</div>
			</div>
		{/snippet}

		{#snippet actions()}
			<div class="flex flex-row gap-2 h-6 -my-2">
				<button
					class="group-hover:opacity-100 opacity-0 py-1 px-2 w-fit duration-200 hover:bg-surface-hover rounded-md border flex items-center justify-center h-full text-gray-500"
				>
					<Plug size={14} />
				</button>
				{@render toggleButton?.()}
			</div>
			{#snippet toggleButton()}
				<div
					class="flex flex-row w-fit border border-nord-500 rounded-md transition-opacity duration-200 group-hover:opacity-100 opacity-0 h-full"
				>
					<button
						class="py-1 px-2 w-fit hover:bg-surface-hover rounded-md h-full text-xs font-normal text-gray-500"
					>
						{'${}'}
					</button>
					<button class=" py-1 px-2 w-fit rounded-md h-full bg-nord-500 text-nord-0">
						<SquareFunction size={14} />
					</button>
				</div>
			{/snippet}
		{/snippet}
	{:else if version == 'minimalistic'}
		<!--
	Accent : nord-0
	Text Neutral : nord-300
	Text Highlight: nord-0 / semibold
	Text Secondary: nord-300/50
	Background surface-secondary: nord-600/20
	Border: nord-500
	callForAction: gray-500
	-->
		<div class="flex flex-col gap-8">
			{#each Object.entries(schema.properties) as [key, value]}
				{@render input({ key, value })}
			{/each}
		</div>

		{#snippet input({ key, value })}
			<div class="flex flex-col gap-1 rounded-md transition-all duration-300 group">
				<!-- Header  -->
				<div class="flex flex-row gap-2 justify-between items-center">
					<div class="flex flex-row gap-2">
						<label for={key} class="text-sm font-semibold text-nord-0 whitespace-nowrap"
							>{key}{#if schema.required.includes(key)}
								<span class="text-sm text-red-500">&nbsp;*</span>
							{/if}
						</label>

						<div class="text-sm text-nord-300/50 italic">{value.type}</div>
					</div>

					{@render actions?.()}
				</div>
				<!-- Input -->
				<input
					type="text"
					class="!rounded-sm !bg-gray-50 !shadow-none !border-none hover:!border-nord-400 !text-nord-300 !focus:outline-none !focus:ring-none !p-3"
					id={key}
					placeholder={value.placeholder}
					value={value.default}
				/>
				<!-- Description -->
				<div class="text-2xs text-gray-400 italic">{value.description}</div>
			</div>
		{/snippet}

		{#snippet actions()}
			<div class="flex flex-row gap-2 h-6 -my-2">
				<button
					class="group-hover:opacity-100 opacity-0 py-1 px-2 w-fit duration-200 hover:bg-surface-hover rounded-md border flex items-center justify-center h-full text-gray-500"
				>
					<Plug size={14} />
				</button>
				{@render toggleButton?.()}
			</div>
			{#snippet toggleButton()}
				<div
					class="flex flex-row w-fit border border-nord-500 rounded-md transition-opacity duration-200 group-hover:opacity-100 opacity-0 h-full"
				>
					<button
						class="py-1 px-2 w-fit hover:bg-surface-hover rounded-md h-full text-xs font-normal text-gray-500"
					>
						{'${}'}
					</button>
					<button class=" py-1 px-2 w-fit rounded-md h-full bg-nord-500 text-nord-0">
						<SquareFunction size={14} />
					</button>
				</div>
			{/snippet}
		{/snippet}
	{/if}
</div>
