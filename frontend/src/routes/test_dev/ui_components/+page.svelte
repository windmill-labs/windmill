<script lang="ts">
	import { page } from '$app/state'
	import { goto } from '$app/navigation'
	import { Plug, SquareFunction, WandSparkles } from 'lucide-svelte'

	const schema = {
		$schema: 'https://json-schema.org/draft/2020-12/schema',
		properties: {
			provider: {
				type: 'object',
				format: 'ai-provider',
				placeholder: 'Select an AI provider',
				options: [
					{ label: 'OpenAI', value: 'openai' },
					{ label: 'Google AI', value: 'google_ai' },
					{ label: 'OpenRouter', value: 'openrouter' }
				]
			},
			user_message: {
				type: 'string',
				placeholder: 'Ex: Hello, how are you?'
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

	// Toggle state for the rounded toggle component
	let toggleState = $state(false)
</script>

<div class="min-h-screen flex">
	<!-- Design Summary Sidebar -->
	<div class="w-80 bg-nord-6/5 border-r border-nord-4/20 p-6 flex flex-col gap-6">
		<div class="space-y-4">
			<div>
				<h3 class="font-medium text-nord-0 mb-2">Theme Selector</h3>
				<select
					bind:value={() => version, (v) => goto(`?version=${v}`)}
					class="w-full bg-white border border-nord-4/30 rounded px-3 py-2 text-sm text-nord-0 focus:outline-none focus:border-nord-8"
				>
					<option value="minimalistic">Minimalistic</option>
					<option value="minimalistic_border">Minimalistic Border</option>
					<option value="dev_clean">Dev Clean</option>
					<option value="sharp_minimal">Sharp Minimal</option>
					<option value="precision">Precision</option>
					<option value="minimal">Minimal</option>
					<option value="border">Border</option>
					<option value="border_nord">Border Nord</option>
					<option value="card_stack">Card Stack</option>
					<option value="inline_compact">Inline Compact</option>
					<option value="dev_minimal">Dev Minimal</option>
				</select>
			</div>

			{@render themeInfo()}
		</div>
	</div>

	<!-- Main Form Area -->
	<div class="flex-1 p-8 max-w-2xl">
		<div class="max-w-xl mx-auto">{@render formContent()}</div>
	</div>
</div>

{#snippet themeInfo()}
	{#if version == 'minimalistic'}
		<div class="bg-white rounded-lg border border-nord-4/20 p-4">
			<h4 class="font-medium text-nord-0 mb-3">Minimalistic</h4>
			<div class="space-y-3 text-sm">
				<div>
					<span class="font-medium text-nord-0">Colors:</span>
					<span class="text-nord-3/70">Nord palette, subtle backgrounds</span>
				</div>
				<div>
					<span class="font-medium text-nord-0">Typography:</span>
					<span class="text-nord-3/70">Clean, readable hierarchy</span>
				</div>
				<div>
					<span class="font-medium text-nord-0">Spacing:</span>
					<span class="text-nord-3/70">Generous padding, breathing room</span>
				</div>
				<div>
					<span class="font-medium text-nord-0">Features:</span>
					<span class="text-nord-3/70">Enhanced toggle, connect emphasis</span>
				</div>
			</div>
		</div>
	{:else if version == 'minimalistic_border'}
		<div class="bg-white rounded-lg border border-nord-4/20 p-4">
			<h4 class="font-medium text-nord-0 mb-3">Minimalistic Border</h4>
			<div class="space-y-3 text-sm">
				<div>
					<span class="font-medium text-nord-0">Approach:</span>
					<span class="text-nord-3/70"
						>Shapes are slightly rounded, Groups are formed by padding and background color, borders
						can be added to mark contrast. Prefer thin borders to shadows. Text have 3 colors:
						primary, secondary, tertiary. Primary is reserved for headers/labels. Buttons have 2
						main styles: border, no background for secondary action, full accent for primary action</span
					>
				</div>
				<div>
					<span class="font-medium text-nord-0">Colors:</span>
					<span class="text-nord-3/70">Nord palette, subtle backgrounds, no shadows</span>
				</div>
				<div>
					<span class="font-medium text-nord-0">Typography:</span>
					<span class="text-nord-3/70">Clean, readable hierarchy</span>
				</div>
				<div>
					<span class="font-medium text-nord-0">Spacing:</span>
					<span class="text-nord-3/70">Generous padding, breathing room</span>
				</div>
				<div>
					<span class="font-medium text-nord-0">Shapes:</span>
					<span class="text-nord-3/70">Slightly Rounded corners</span>
				</div>
			</div>
		</div>
	{:else if version == 'sharp_minimal'}
		<div class="bg-white rounded-lg border border-nord-4/20 p-4">
			<h4 class="font-medium text-nord-0 mb-3">Sharp Minimal</h4>
			<div class="space-y-3 text-sm">
				<div>
					<span class="font-medium text-nord-0">Design:</span>
					<span class="text-nord-3/70">Crisp edges, high contrast</span>
				</div>
				<div>
					<span class="font-medium text-nord-0">Interaction:</span>
					<span class="text-nord-3/70">Bold connect button, clear states</span>
				</div>
				<div>
					<span class="font-medium text-nord-0">Layout:</span>
					<span class="text-nord-3/70">Tight spacing, maximum efficiency</span>
				</div>
			</div>
		</div>
	{:else if version == 'precision'}
		<div class="bg-white rounded-lg border border-nord-4/20 p-4">
			<h4 class="font-medium text-nord-0 mb-3">Precision</h4>
			<div class="space-y-3 text-sm">
				<div>
					<span class="font-medium text-nord-0">Approach:</span>
					<span class="text-nord-3/70">Data-driven, exact alignment</span>
				</div>
				<div>
					<span class="font-medium text-nord-0">Visual:</span>
					<span class="text-nord-3/70">Grid-based, mathematical spacing</span>
				</div>
				<div>
					<span class="font-medium text-nord-0">Interaction:</span>
					<span class="text-nord-3/70">Precise controls, clear feedback</span>
				</div>
			</div>
		</div>
	{:else}
		<div class="bg-gray-50 rounded-lg border border-gray-200 p-4">
			<h4 class="font-medium text-gray-700 mb-2"
				>{version.charAt(0).toUpperCase() + version.slice(1).replace('_', ' ')}</h4
			>
			<p class="text-sm text-gray-600">Legacy design variation</p>
		</div>
	{/if}
{/snippet}

{#snippet formContent()}
	{#if version == 'minimalistic'}
		<!-- Original Minimalistic Theme -->
		<div class="space-y-8">
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
	{:else if version == 'minimalistic_border'}
		<!-- Original Minimalistic Theme -->
		<div class="flex divide-x divide-gray-200 gap-8">
			<div class="flex flex-col gap-2 min-w-[600px]">
				{#each Object.entries(schema.properties) as [key, value]}
					{@render input({ key, value })}
				{/each}
			</div>
			<div class="flex flex-col gap-8 px-4 min-w-[600px]">
				<div class="flex flex-row gap-2 text-sm text-nord-0"
					>toggle:{@render toggleButton?.(true)}</div
				>
				<div class="flex flex-row gap-2 text-sm text-nord-0"
					>buttons light most of the buttons:
					<button
						class="py-1 px-2 w-fit duration-200 hover:bg-surface-hover rounded-md border flex items-center justify-center h-full text-gray-500"
					>
						<Plug size={14} />
					</button>
				</div>
				<div class="flex flex-row gap-2 text-sm text-nord-0"
					>buttons full accent for the main call to action only:
					<button
						class="text-white py-1 px-2 w-fit duration-200 rounded-md justify-center h-full bg-nord-950 hover:bg-[#4d698b]"
					>
						Click me !
					</button>
				</div>

				<div class="flex flex-row gap-2 text-sm text-nord-0 items-center">
					rounded toggle:
					<button
						class="relative w-9 h-5 rounded-full transition-all duration-100 {toggleState
							? 'bg-nord-950 hover:bg-[#4d698b]'
							: 'bg-nord-600 hover:bg-nord-500'}"
						aria-label="Toggle switch"
						onclick={() => (toggleState = !toggleState)}
					>
						<div
							class="absolute top-0 left-0 w-5 h-5 rounded-full bg-white transition-transform duration-100 ease-in-out border border-gray-300 {toggleState
								? 'translate-x-4 border-nord-950'
								: ''}"
						></div>
					</button>
				</div>

				<span class="text-sm text-nord-0"> text primary : nord-0 for headers/labels </span>
				<span class="text-sm text-nord-300">
					text secondary : nord-300 for most of the text and input values
				</span>
				<span class="text-sm text-gray-400">
					text tertiary : gray-400 for placeholders/descriptions
				</span>
				<div class="flex flex-row gap-2 text-sm text-nord-0">
					input with placeholder:
					<input
						type="text"
						class="windmillapp !bg-gray-50 rounded-sm border border-transparent hover:border-outline-400 text-nord-300 p-2 placeholder-gray-400 text-sm focus:ring-0 focus:outline-none focus:border-nord-900"
						placeholder={'Ex: placeholder'}
					/>
				</div>
			</div>
		</div>
		{#snippet input({ key, value })}
			<div
				class="flex flex-col gap-1.5 rounded-md transition-all duration-300 group px-4 py-6 border border-transparent hover:border-nord-600"
			>
				<!-- Header  -->
				<div class="flex flex-row gap-2 justify-between items-center">
					<div class="flex flex-row gap-2">
						<label for={key} class="text-sm font-semibold text-nord-0 whitespace-nowrap"
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
					class="windmillapp !bg-gray-50 rounded-sm border border-transparent hover:border-outline-400 text-nord-300 p-2 placeholder-gray-400 text-sm focus:ring-0 focus:outline-none focus:border-nord-900"
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
					class="group-hover:opacity-100 opacity-0 py-1 px-2 w-fit duration-200 hover:bg-surface-hover rounded-md border flex items-center justify-center h-full text-purple-500 border-purple-200"
				>
					<WandSparkles size={14} />
				</button>
				{@render toggleButton?.()}
				<button
					class="group-hover:opacity-100 opacity-0 py-1 px-2 w-fit duration-200 hover:bg-surface-hover rounded-md border flex items-center justify-center h-full text-gray-500"
				>
					<Plug size={14} />
				</button>
			</div>
		{/snippet}
		{#snippet toggleButton(forceVisible = false)}
			<div
				class="h-6 flex flex-row w-fit rounded-md transition-opacity duration-200 group-hover:opacity-100 {forceVisible
					? 'opacity-100'
					: 'opacity-0'} bg-nord-600 items-center"
			>
				<button
					class="py-1 px-2 w-fit hover:bg-surface-hover rounded-md h-full text-2xs font-normal text-gray-400 hover:text-nord-0 center-center"
				>
					static
				</button>
				<button
					class=" py-1 px-2 w-fit rounded-md h-full bg-gray-50 text-nord-0 border-gray-300 border"
				>
					<SquareFunction size={12} />
				</button>
			</div>
		{/snippet}
	{:else if version == 'sharp_minimal'}
		<!-- Sharp Minimal - High contrast, crisp edges, maximum efficiency -->
		<div class="space-y-4">
			{#each Object.entries(schema.properties) as [key, value]}
				{@render input({ key, value })}
			{/each}
		</div>

		{#snippet input({ key, value })}
			<div
				class="group bg-white border border-nord-4/40 hover:border-nord-8/60 transition-colors duration-150"
			>
				<div class="flex items-center justify-between px-4 py-3 border-b border-nord-4/20">
					<div class="flex items-center gap-3">
						<label for={key} class="text-sm font-semibold text-nord-0 uppercase tracking-wide">
							{key}
						</label>
						{#if schema.required.includes(key)}
							<span class="text-xs bg-nord-11 text-white px-2 py-0.5 rounded font-bold">REQ</span>
						{/if}
						<code class="text-xs text-nord-3/70 bg-nord-6/10 px-2 py-1 rounded font-mono">
							{value.type}
						</code>
					</div>
					{@render toggle()}
				</div>
				<div class="px-4 py-3">
					<input
						type="text"
						class="w-full border-0 bg-transparent text-nord-0 placeholder-nord-3/50 focus:outline-none text-base"
						id={key}
						placeholder={value.placeholder}
						value={value.default}
					/>
				</div>
				{#if value.description}
					<div class="px-4 pb-3">
						<p class="text-xs text-nord-3/70">{value.description}</p>
					</div>
				{/if}
			</div>
		{/snippet}

		{#snippet toggle()}
			<div class="flex items-center gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
				<!-- Connect Button -->
				<button
					class="bg-nord-8 text-white px-3 py-1 rounded text-xs font-bold hover:bg-nord-9 transition-colors flex items-center gap-1"
					title="Connect this field to a resource or variable"
				>
					<Plug size={12} />
					CONNECT
				</button>
				<!-- Toggle between Static Input (${}) and JavaScript Expression (Function) -->
				<div
					class="flex border border-nord-4/40 rounded overflow-hidden"
					role="group"
					aria-label="Input type toggle"
				>
					<button
						class="px-2 py-1 text-xs bg-nord-6/20 text-nord-0 hover:bg-nord-6/30 transition-colors font-medium"
						title="Static input with variable insertion"
						aria-pressed="true"
					>
						$&#123;&#125;
					</button>
					<button
						class="px-2 py-1 text-xs bg-white text-nord-3/80 hover:bg-nord-6/20 transition-colors border-l border-nord-4/40"
						title="JavaScript expression"
						aria-pressed="false"
					>
						<SquareFunction size={10} />
					</button>
				</div>
			</div>
		{/snippet}
	{:else if version == 'precision'}
		<!-- Precision - Clean grid-based layout with perfect alignment -->
		<div class="space-y-1">
			{#each Object.entries(schema.properties) as [key, value]}
				{@render input({ key, value })}
			{/each}
		</div>

		{#snippet input({ key, value })}
			<div
				class="group grid grid-cols-12 gap-6 items-center py-4 px-4 hover:bg-nord-6/5 rounded-md transition-colors"
			>
				<!-- Label Column (3 units) - Clean alignment -->
				<div class="col-span-3">
					<label for={key} class="text-sm font-medium text-nord-0 flex items-center gap-2">
						{key}
						{#if schema.required.includes(key)}
							<div class="w-1.5 h-1.5 bg-nord-11 rounded-full flex-shrink-0"></div>
						{/if}
					</label>
					<div class="text-xs text-nord-3/60 font-mono mt-0.5">{value.type}</div>
				</div>

				<!-- Input Column (7 units) - Generous space -->
				<div class="col-span-7">
					<input
						type="text"
						class="w-full bg-nord-6/8 border border-nord-4/30 rounded-md px-3 py-2.5 text-nord-0 placeholder-nord-3/50 focus:outline-none focus:border-nord-8 focus:bg-white transition-all duration-150 text-sm"
						id={key}
						placeholder={value.placeholder}
						value={value.default}
					/>
					{#if value.description}
						<p class="text-xs text-nord-3/60 mt-1.5 leading-relaxed">{value.description}</p>
					{/if}
				</div>

				<!-- Controls Column (2 units) - Right aligned -->
				<div class="col-span-2 flex justify-end">
					{@render toggle()}
				</div>
			</div>
		{/snippet}

		{#snippet toggle()}
			<div
				class="flex items-center gap-1.5 opacity-0 group-hover:opacity-100 transition-opacity duration-150"
			>
				<!-- Connect button -->
				<button
					class="bg-nord-8 text-white p-2 rounded-md hover:bg-nord-9 transition-colors shadow-sm"
					title="Connect this field"
				>
					<Plug size={12} />
				</button>

				<!-- Toggle controls - horizontal layout -->
				<div
					class="flex border border-nord-4/30 rounded-md overflow-hidden bg-white shadow-sm"
					role="group"
					aria-label="Input type"
				>
					<button
						class="p-2 text-xs bg-nord-6/15 text-nord-0 hover:bg-nord-6/25 transition-colors font-medium"
						title="Static input with variables"
						aria-pressed="true"
					>
						{'${}'}
					</button>
					<button
						class="p-2 bg-white text-nord-3/80 hover:bg-nord-6/15 transition-colors border-l border-nord-4/30"
						title="JavaScript expression"
						aria-pressed="false"
					>
						<SquareFunction size={10} />
					</button>
				</div>
			</div>
		{/snippet}
	{:else if version == 'minimal'}
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
			<div
				class="bg-white shadow-md rounded-lg border border-gray-100 hover:shadow-lg transition-shadow duration-200 p-5"
			>
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
				<button
					class="p-1.5 text-nord-3/60 hover:text-nord-8 hover:bg-nord-6/20 rounded transition-colors"
				>
					<Plug size={14} />
				</button>
				<button
					class="p-1.5 text-nord-3/60 hover:text-nord-10 hover:bg-nord-6/20 rounded transition-colors"
				>
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
					class="w-full !rounded-none !shadow-none !border-0 !border-b !border-nord-4/30 !bg-transparent !pb-2 !text-nord-0 !placeholder-nord-3/40 !focus:outline-none !focus:border-nord-8 transition-colors"
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
{/snippet}
