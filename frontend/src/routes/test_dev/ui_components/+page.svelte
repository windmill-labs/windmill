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
		<option value="card_stack">Card Stack</option>
		<option value="inline_compact">Inline Compact</option>
		<option value="dev_minimal">Dev Minimal</option>
		<option value="dev_clean">Dev Clean</option>
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
	{:else if version == 'card_stack'}
		<!-- Card Stack Layout - Each field is a distinct card -->
		<div class="flex flex-col gap-3">
			{#each Object.entries(schema.properties) as [key, value]}
				{@render input({ key, value })}
			{/each}
		</div>

		{#snippet input({ key, value })}
			<div class="bg-white shadow-md rounded-lg border border-gray-100 hover:shadow-lg transition-shadow duration-200 p-5">
				<!-- Label with Icon -->
				<div class="flex items-center gap-2 mb-3">
					<div class="w-2 h-2 rounded-full bg-blue-500"></div>
					<label for={key} class="text-base font-medium text-gray-900">
						{key}{#if schema.required.includes(key)}
							<span class="text-red-500 ml-1">*</span>
						{/if}
					</label>
					<div class="text-xs px-2 py-1 bg-gray-100 rounded-full text-gray-600 font-mono">
						{value.type}
					</div>
				</div>
				<!-- Input -->
				<input
					type="text"
					class="w-full border border-gray-200 rounded-md px-3 py-2.5 text-gray-700 focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500 transition-colors"
					id={key}
					placeholder={value.placeholder ?? value.description}
					value={value.default}
				/>
				<!-- Description -->
				{#if value.description}
					<p class="text-sm text-gray-500 mt-2 leading-relaxed">{value.description}</p>
				{/if}
			</div>
		{/snippet}
	{:else if version == 'inline_compact'}
		<!-- Inline Compact - Labels and inputs on same line -->
		<div class="space-y-4">
			{#each Object.entries(schema.properties) as [key, value]}
				{@render input({ key, value })}
			{/each}
		</div>

		{#snippet input({ key, value })}
			<div class="grid grid-cols-3 gap-4 items-start py-3 border-b border-gray-100 last:border-b-0">
				<!-- Label Column -->
				<div class="text-right pr-3">
					<label for={key} class="text-sm font-medium text-gray-700 block">
						{key}{#if schema.required.includes(key)}
							<span class="text-red-500">*</span>
						{/if}
					</label>
					<div class="text-xs text-gray-400 mt-1">{value.type}</div>
				</div>
				<!-- Input Column -->
				<div class="col-span-2">
					<input
						type="text"
						class="w-full border border-gray-200 rounded-sm px-3 py-2 text-sm text-gray-700 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
						id={key}
						placeholder={value.placeholder}
						value={value.default}
					/>
					{#if value.description}
						<p class="text-xs text-gray-500 mt-1">{value.description}</p>
					{/if}
				</div>
			</div>
		{/snippet}
	{:else if version == 'dev_minimal'}
		<!-- Developer Minimal - Ultra clean with Nord colors -->
		<div class="space-y-6">
			{#each Object.entries(schema.properties) as [key, value]}
				{@render input({ key, value })}
			{/each}
		</div>

		{#snippet input({ key, value })}
			<div class="group">
				<div class="flex items-center justify-between mb-2">
					<label for={key} class="text-sm font-medium text-nord-0">
						{key}{#if schema.required.includes(key)}
							<span class="text-nord-11 ml-1">*</span>
						{/if}
						<span class="text-nord-3/60 font-mono text-xs ml-2">{value.type}</span>
					</label>
					{@render actions()}
				</div>
				<input
					type="text"
					class="w-full bg-nord-6/10 border border-nord-4/20 rounded px-3 py-2 text-nord-0 placeholder-nord-3/50 focus:outline-none focus:border-nord-8 focus:bg-nord-6/5 transition-colors"
					id={key}
					placeholder={value.placeholder}
					value={value.default}
				/>
				{#if value.description}
					<p class="text-xs text-nord-3/60 mt-1">{value.description}</p>
				{/if}
			</div>
		{/snippet}

		{#snippet actions()}
			<div class="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
				<button class="p-1.5 text-nord-3/60 hover:text-nord-8 hover:bg-nord-6/20 rounded transition-colors">
					<Plug size={14} />
				</button>
				<button class="p-1.5 text-nord-3/60 hover:text-nord-10 hover:bg-nord-6/20 rounded transition-colors">
					<SquareFunction size={14} />
				</button>
			</div>
		{/snippet}
	{:else if version == 'dev_clean'}
		<!-- Developer Clean - Extremely minimal, focus on content -->
		<div class="space-y-8">
			{#each Object.entries(schema.properties) as [key, value]}
				{@render input({ key, value })}
			{/each}
		</div>

		{#snippet input({ key, value })}
			<div class="group">
				<div class="flex items-baseline justify-between mb-1">
					<div class="flex items-baseline gap-3">
						<label for={key} class="text-nord-0 font-medium">
							{key}
						</label>
						<span class="text-nord-3/50 text-xs font-mono">{value.type}</span>
						{#if schema.required.includes(key)}
							<span class="w-1 h-1 bg-nord-11 rounded-full"></span>
						{/if}
					</div>
					{@render actions()}
				</div>
				<input
					type="text"
					class="w-full border-0 border-b border-nord-4/30 bg-transparent pb-2 text-nord-0 placeholder-nord-3/40 focus:outline-none focus:border-nord-8 transition-colors"
					id={key}
					placeholder={value.placeholder}
					value={value.default}
				/>
				{#if value.description}
					<p class="text-xs text-nord-3/50 mt-2">{value.description}</p>
				{/if}
			</div>
		{/snippet}

		{#snippet actions()}
			<div class="flex gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
				<button class="text-nord-3/40 hover:text-nord-8 transition-colors" title="Connect">
					<Plug size={14} />
				</button>
				<button class="text-nord-3/40 hover:text-nord-10 transition-colors" title="Function">
					<SquareFunction size={14} />
				</button>
			</div>
		{/snippet}
	{/if}
</div>
