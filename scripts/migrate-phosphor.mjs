#!/usr/bin/env node
/**
 * Codemod: migrate phosphor-svelte v3 deprecated aliases
 * -------------------------------------------------------
 * `phosphor-svelte@3` deprecated every unsuffixed default export (e.g.
 * `ShieldCheck`) in favor of an `Icon`-suffixed twin (`ShieldCheckIcon`).
 *
 * This script rewrites, across `src/**`:
 *   1. Import paths:  `phosphor-svelte/lib/ShieldCheck` → `phosphor-svelte/lib/ShieldCheckIcon`
 *   2. Identifiers:   every usage of the imported name is renamed with a `\b`
 *      word-boundary regex so partial collisions (e.g. `Users` in `UsersList`)
 *      are never mutated.
 *
 * Idempotent — re-running on already-migrated files is a no-op.
 *
 * Usage:   node scripts/migrate-phosphor.mjs
 */
import { promises as fs } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SRC = path.resolve(__dirname, '..', 'src');

/** Matches lines like:
 *    import ShieldCheck from 'phosphor-svelte/lib/ShieldCheck';
 *    import ShieldCheck from "phosphor-svelte/lib/ShieldCheck";
 *  Skips lines that already end with `Icon`.
 */
const IMPORT_RE =
	/import\s+(\w+)\s+from\s+['"]phosphor-svelte\/lib\/([A-Z]\w*?)['"];?/g;

async function walk(dir) {
	const entries = await fs.readdir(dir, { withFileTypes: true });
	const out = [];
	for (const entry of entries) {
		const full = path.join(dir, entry.name);
		if (entry.isDirectory()) {
			out.push(...(await walk(full)));
		} else if (entry.isFile() && /\.(svelte|ts|js)$/.test(entry.name)) {
			out.push(full);
		}
	}
	return out;
}

let totalFiles = 0;
let totalRenames = 0;

const files = await walk(SRC);

for (const file of files) {
	const original = await fs.readFile(file, 'utf8');
	if (!original.includes('phosphor-svelte/lib/')) continue;

	/** @type {Array<[string, string]>} */
	const renames = [];
	let updated = original.replace(IMPORT_RE, (match, id, moduleName) => {
		if (moduleName.endsWith('Icon')) return match; // already migrated
		const newId = `${id}Icon`;
		const newModule = `${moduleName}Icon`;
		renames.push([id, newId]);
		return `import ${newId} from 'phosphor-svelte/lib/${newModule}';`;
	});

	if (renames.length === 0) continue;

	for (const [oldId, newId] of renames) {
		// Word-boundary replace so `Clock` does not clobber `OnClock` etc.
		const wordRe = new RegExp(`\\b${oldId}\\b`, 'g');
		updated = updated.replace(wordRe, newId);
		totalRenames++;
	}

	if (updated !== original) {
		await fs.writeFile(file, updated, 'utf8');
		totalFiles++;
		console.log(`✓ ${path.relative(path.resolve(__dirname, '..'), file)}  (${renames.length} icon${renames.length === 1 ? '' : 's'})`);
	}
}

console.log(`\nDone. Migrated ${totalRenames} imports across ${totalFiles} file(s).`);
