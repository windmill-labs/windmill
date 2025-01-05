import * as idb from 'idb-keyval'
import type { InstalledPackage } from './npm_install'
// import { is_client } from './dom'

const NPM_PACKAGES = idb.createStore('npm', 'packages')

function key(name: string, version: string) {
	return `${name}@${version}`
}
export function get(name: string, version: string) {
	return idb.get<InstalledPackage>(key(name, version), NPM_PACKAGES)
}

export function set(name: string, version: string, pkg: InstalledPackage) {
	return idb.set(key(name, version), pkg, NPM_PACKAGES)
}
