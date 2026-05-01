import prettier from 'eslint-config-prettier';
import path from 'node:path';
import { includeIgnoreFile } from '@eslint/compat';
import js from '@eslint/js';
import svelte from 'eslint-plugin-svelte';
import { defineConfig } from 'eslint/config';
import globals from 'globals';
import ts from 'typescript-eslint';
import svelteConfig from './svelte.config.js';

const gitignorePath = path.resolve(import.meta.dirname, '.gitignore');

export default defineConfig(
	includeIgnoreFile(gitignorePath),
	{
		// Rust build artifacts (including vendored swagger-ui JS from `utoipa-swagger-ui`)
		// and rendered OpenAPI artifacts. These are generated, not authored.
		ignores: ['backend/target/**', 'backend/tests/snapshots/**']
	},
	js.configs.recommended,
	...ts.configs.recommended,
	...svelte.configs.recommended,
	prettier,
	...svelte.configs.prettier,
	{
		languageOptions: { globals: { ...globals.browser, ...globals.node } },
		rules: {
			// typescript-eslint strongly recommend that you do not use the no-undef lint rule on TypeScript projects.
			// see: https://typescript-eslint.io/troubleshooting/faqs/eslint/#i-get-errors-from-the-no-undef-rule-about-global-variables-not-being-defined-even-though-there-are-no-typescript-errors
			'no-undef': 'off',
			'@typescript-eslint/no-unused-vars': [
				'warn',
				{ argsIgnorePattern: '^_', varsIgnorePattern: '^_' }
			]
		}
	},
	{
		files: ['**/*.svelte', '**/*.svelte.ts', '**/*.svelte.js'],
		languageOptions: {
			parserOptions: {
				projectService: true,
				extraFileExtensions: ['.svelte'],
				parser: ts.parser,
				svelteConfig
			}
		},
		rules: {
			// `require-each-key` (each block keys), `no-dom-manipulating`, `no-at-html-tags` and
			// `no-unused-svelte-ignore` are all back at their default `error` level. Every existing
			// `{@html …}` site has a targeted `eslint-disable-next-line` comment with a one-line WHY:
			// content is either statically authored marketing copy or sanitized via DOMPurify in
			// `$lib/utils/safeHtml`.
			//
			// `svelte/no-navigation-without-resolve` is intentionally held at `warn`. The SvelteKit 2
			// `resolve()` migration is a 232-warning / 73-file mechanical sweep — promote to `error`
			// once the sweep ships.
			'svelte/no-navigation-without-resolve': 'warn',
			'svelte/require-each-key': 'error',
			'svelte/no-dom-manipulating': 'error',
			'svelte/no-at-html-tags': 'error',
			'svelte/no-unused-svelte-ignore': 'error'
		}
	}
);
