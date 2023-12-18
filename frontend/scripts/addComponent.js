import fs from 'fs'
import path from 'path'
import { fileURLToPath } from 'url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

const componentsFilePath = path.join(
	__dirname,
	'../src/lib/components/apps/editor/component/components.ts'
)

function addComponentType(componentName) {
	const typeName = componentName.charAt(0).toUpperCase() + componentName.slice(1) + 'Component'
	const componentTypeString = `export type ${typeName} = BaseComponent<'${componentName.toLowerCase()}'>\n`
	const typedComponentString = `\t| ${typeName}\n`
	const componentObjectString = createComponentObjectString(componentName)

	let fileContent = fs.readFileSync(componentsFilePath, 'utf8')

	// Add to TypedComponent
	const typedComponentStart = fileContent.indexOf('export type TypedComponent =')
	const typedComponentEnd = fileContent.indexOf('\n', typedComponentStart)
	fileContent =
		fileContent.slice(0, typedComponentEnd) +
		typedComponentString +
		fileContent.slice(typedComponentEnd)

	// Add above TypeAnchor
	const anchor = '// #TypeAnchor'
	const anchorIndex = fileContent.indexOf(anchor)
	if (anchorIndex === -1) {
		console.error('Anchor not found in the file.')
		process.exit(1)
	}
	fileContent =
		fileContent.slice(0, anchorIndex) + componentTypeString + fileContent.slice(anchorIndex)

	// Add to components object
	const componentsEndIndex = fileContent.lastIndexOf('}')
	fileContent =
		fileContent.slice(0, componentsEndIndex) +
		componentObjectString +
		fileContent.slice(componentsEndIndex)

	fs.writeFileSync(componentsFilePath, fileContent)
}

function createComponentObjectString(componentName) {
	const lowerCaseName = componentName.toLowerCase()
	return `,
\t${lowerCaseName}component: {
\t\tname: '${componentName}',
\t\ticon: BoxSelect,
\t\tdocumentationLink: \`\${documentationBaseUrl}/${lowerCaseName}\`,
\t\tdims: '2:8-6:8' as AppComponentDimensions,
\t\tcustomCss: {
\t\t\tcontainer: { class: '', style: '' }
\t\t},
\t\tinitialData: {
\t\t\tconfiguration: {},
\t\t\tcomponentInput: undefined
\t\t}
\t},\n`
}

// Parse command line arguments
const args = process.argv.slice(2)
const componentName = args[0]

if (!componentName) {
	console.error('Please provide a component name.')
	process.exit(1)
}

addComponentType(componentName)
console.log(`Component type ${componentName} added successfully.`)
