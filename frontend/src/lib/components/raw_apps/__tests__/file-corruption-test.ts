/**
 * Test to investigate file corruption issue when AI sets frontend files
 *
 * The issue: When AI calls set_frontend_file with correct content,
 * the file gets corrupted with duplicated/mangled code.
 *
 * Hypothesis: The iframe postMessage roundtrip is corrupting the data.
 */

interface SetFilesMessage {
	type: 'setFiles'
	files: Record<string, string>
}

/**
 * Simulates what happens when AI sets a file
 */
export function simulateAiSetFile(
	path: string,
	content: string,
	existingFiles: Record<string, string>
): {
	sentToIframe: Record<string, string>
	log: string[]
} {
	const log: string[] = []

	// Step 1: AI tool calls setFrontendFile
	log.push(`[AI] Setting file: ${path} (${content.length} chars)`)

	// Step 2: Update files object
	const files = { ...existingFiles }
	files[path] = content
	log.push(`[State] Files object updated`)

	// Step 3: Send to iframe
	const filesToSend = Object.fromEntries(Object.entries(files).filter(([p, _]) => !p.endsWith('/')))
	log.push(`[Send] Sending ${Object.keys(filesToSend).length} files to iframe`)

	return {
		sentToIframe: filesToSend,
		log
	}
}

/**
 * Simulates what the iframe might do when it receives setFiles
 * This is where corruption might be happening
 */
export function simulateIframeProcessing(receivedFiles: Record<string, string>): {
	echoedBack: Record<string, string>
	log: string[]
} {
	const log: string[] = []
	log.push(`[Iframe] Received ${Object.keys(receivedFiles).length} files`)

	// The iframe might:
	// 1. Parse the files
	// 2. Load into Monaco editor
	// 3. Format/process somehow
	// 4. Echo back to parent

	// TODO: This is where we need to investigate what actually happens
	// For now, just echo back unchanged
	const echoedBack = { ...receivedFiles }

	log.push(`[Iframe] Echoing back ${Object.keys(echoedBack).length} files`)

	return {
		echoedBack,
		log
	}
}

/**
 * Test the full flow
 */
export function testFileSetFlow() {
	const originalContent = `<script lang="ts">
  import { backend } from "./wmill";
  import { onMount } from "svelte";

  interface Todo {
    id: number;
    text: string;
    completed: boolean;
    created_at: string;
    updated_at: string;
  }

  let todos: Todo[] = [];
  let newTodoText = "";
  let loading = false;

  onMount(async () => {
    await loadTodos();
  });

  async function loadTodos() {
    try {
      loading = true;
      todos = await backend.getTodos();
    } catch (error) {
      console.error("Failed to load todos:", error);
    } finally {
      loading = false;
    }
  }
</script>

<main>
  <h1>Todo List</h1>
</main>

<style>
  .container {
    max-width: 600px;
  }
</style>`

	const existingFiles = {
		'/index.tsx': 'export default function App() {}',
		'/wmill.d.ts': 'export const backend: any'
	}

	console.log('=== Testing File Corruption Flow ===\n')

	// Step 1: AI sets the file
	const { sentToIframe, log: aiLog } = simulateAiSetFile(
		'/App.svelte',
		originalContent,
		existingFiles
	)

	aiLog.forEach((line) => console.log(line))

	console.log('\n--- Content sent to iframe ---')
	console.log('Path: /App.svelte')
	console.log('Length:', sentToIframe['/App.svelte']?.length)
	console.log('First 100 chars:', sentToIframe['/App.svelte']?.substring(0, 100))

	// Step 2: Iframe processes and echoes back
	const { echoedBack, log: iframeLog } = simulateIframeProcessing(sentToIframe)

	console.log('\n')
	iframeLog.forEach((line) => console.log(line))

	console.log('\n--- Content echoed back ---')
	console.log('Path: /App.svelte')
	console.log('Length:', echoedBack['/App.svelte']?.length)
	console.log('First 100 chars:', echoedBack['/App.svelte']?.substring(0, 100))

	// Step 3: Compare
	const original = sentToIframe['/App.svelte']
	const echoed = echoedBack['/App.svelte']

	console.log('\n=== Comparison ===')
	console.log('Lengths match:', original.length === echoed.length)
	console.log('Content match:', original === echoed)

	if (original !== echoed) {
		console.log('\n🔴 CORRUPTION DETECTED!')
		console.log('Original length:', original.length)
		console.log('Echoed length:', echoed.length)

		// Find first difference
		for (let i = 0; i < Math.min(original.length, echoed.length); i++) {
			if (original[i] !== echoed[i]) {
				console.log(`First difference at position ${i}:`)
				console.log(`  Original: "${original.substring(i, i + 50)}"`)
				console.log(`  Echoed:   "${echoed.substring(i, i + 50)}"`)
				break
			}
		}
	} else {
		console.log('\n✅ No corruption in test simulation')
		console.log('(This means corruption happens in actual iframe, not in our data flow)')
	}
}

/**
 * Helper to detect corruption patterns in content
 */
export function detectCorruptionPatterns(content: string): {
	duplicatedLines: number
	randomChars: number
	brokenSyntax: boolean
	patterns: string[]
} {
	const lines = content.split('\n')
	const patterns: string[] = []

	// Check for duplicated lines
	const lineSet = new Set(lines)
	const duplicatedLines = lines.length - lineSet.size
	if (duplicatedLines > lines.length * 0.2) {
		patterns.push(`High duplication: ${duplicatedLines}/${lines.length} lines`)
	}

	// Check for character corruption (random missing chars)
	const randomCharMatches = content.match(/\w{2,}[^\w\s]{1}\w{2,}/g) || []
	const randomChars = randomCharMatches.length
	if (randomChars > 10) {
		patterns.push(`Possible char corruption: ${randomChars} instances`)
	}

	// Check for broken syntax (unmatched brackets)
	const openBraces = (content.match(/\{/g) || []).length
	const closeBraces = (content.match(/\}/g) || []).length
	const openParens = (content.match(/\(/g) || []).length
	const closeParens = (content.match(/\)/g) || []).length
	const brokenSyntax =
		Math.abs(openBraces - closeBraces) > 5 || Math.abs(openParens - closeParens) > 5
	if (brokenSyntax) {
		patterns.push(`Unbalanced: {${openBraces}/${closeBraces} }${openParens}/${closeParens}`)
	}

	return {
		duplicatedLines,
		randomChars,
		brokenSyntax,
		patterns
	}
}

// Run the test
if (typeof window === 'undefined') {
	testFileSetFlow()
}
