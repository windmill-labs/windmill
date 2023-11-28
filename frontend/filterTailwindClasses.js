import fs from 'fs'
import tailwindClasses from './tailwindClasses.js'

function filterTailwindClasses(classes) {
	const filters = [
		{ pattern: /^m(\w?)-.*$/ },
		{ pattern: /^p(\w?)-.*$/ },
		{ pattern: /^rounded-.*$/ },
		{ pattern: /^shadow-.*$/ },
		{ pattern: /^text-[^/]*$/ },
		{ pattern: /^bg-[^/]*$/ },
		{ pattern: /^border-[^/]*$/ },
		{ pattern: /^ring-[^/]*$/ }
	]

	return classes.filter((className) => {
		return filters.some((filter) => filter.pattern.test(className))
	})
}

const filteredClasses = filterTailwindClasses(tailwindClasses)

const output = `export const filteredTailwindClasses = ${JSON.stringify(filteredClasses, null, 2)};`
fs.writeFileSync('filteredTailwindClasses.ts', output)
