import { execSync } from 'child_process'
import { writeFileSync, mkdirSync, cpSync, existsSync, rmSync, readdirSync, readFileSync } from 'fs'
import { fileURLToPath } from 'url'
import { dirname, join, resolve, relative } from 'path'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

const frontendDir = resolve(__dirname, '..')
const rootDir = resolve(frontendDir, '..')
const systemPromptsDir = join(rootDir, 'system_prompts', 'auto-generated')
const packageDir = join(frontendDir, 'package')
const packageSystemPromptsDir = join(packageDir, 'system_prompts')

// Create package/system_prompts directory
if (!existsSync(packageSystemPromptsDir)) {
	mkdirSync(packageSystemPromptsDir, { recursive: true })
}

// Create a temporary tsconfig for compiling system_prompts
const tempTsConfig = {
	compilerOptions: {
		target: 'ES2021',
		module: 'ESNext',
		moduleResolution: 'bundler',
		lib: ['ES2021'],
		declaration: true,
		declarationMap: false,
		sourceMap: false,
		outDir: packageSystemPromptsDir,
		rootDir: systemPromptsDir,
		esModuleInterop: true,
		skipLibCheck: true,
		strict: false,
		isolatedModules: true,
		resolveJsonModule: true,
		allowSyntheticDefaultImports: true
	},
	include: [join(systemPromptsDir, '**/*.ts')],
	exclude: ['node_modules']
}

const tempTsConfigPath = join(frontendDir, 'tsconfig.system-prompts.json')
writeFileSync(tempTsConfigPath, JSON.stringify(tempTsConfig, null, 2))

try {
	// Compile TypeScript files
	console.log('Compiling system_prompts...')
	execSync(`npx tsc --project ${tempTsConfigPath}`, {
		stdio: 'inherit',
		cwd: frontendDir
	})

	console.log('system_prompts compiled and copied to package/system_prompts')

	// Rewrite imports in packaged files to use the correct path
	console.log('Rewriting $system_prompts imports in packaged files...')

	function rewriteImportsInFile(filePath) {
		const content = readFileSync(filePath, 'utf-8')

		// Specifically rewrite: ../../../../../../../system_prompts/auto-generated to ../../../../system_prompts
		const pattern = /from\s+['"](\.\.\/){7}system_prompts\/auto-generated['"]/g

		const modified = content.replace(pattern, "from '../../../../system_prompts'")

		if (modified !== content) {
			writeFileSync(filePath, modified, 'utf-8')
		}
	}

	function processDirectory(dir) {
		const entries = readdirSync(dir, { withFileTypes: true })
		for (const entry of entries) {
			const fullPath = join(dir, entry.name)
			if (entry.isDirectory()) {
				processDirectory(fullPath)
			} else if (entry.isFile() && entry.name.endsWith('.js')) {
				rewriteImportsInFile(fullPath)
			}
		}
	}

	processDirectory(packageDir)
	console.log('Finished rewriting imports')
} finally {
	// Clean up temporary tsconfig
	if (existsSync(tempTsConfigPath)) {
		rmSync(tempTsConfigPath)
	}
}
