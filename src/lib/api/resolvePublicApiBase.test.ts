import { describe, it, expect } from 'vitest';
import { resolvePublicApiBase } from './resolvePublicApiBase';

describe('resolvePublicApiBase', () => {
	it('server-side: trims VITE value and strips trailing slash', () => {
		expect(
			resolvePublicApiBase({
				viteApiUrl: ' https://api.example.com/ ',
				dev: false,
				browser: false
			})
		).toBe('https://api.example.com');
	});

	it('dev + browser: empty string for Vite proxy', () => {
		expect(
			resolvePublicApiBase({
				viteApiUrl: undefined,
				dev: true,
				browser: true
			})
		).toBe('');
	});

	it('dev + server: loopback for direct Rust access', () => {
		expect(
			resolvePublicApiBase({
				viteApiUrl: undefined,
				dev: true,
				browser: false
			})
		).toBe('http://127.0.0.1:3001');
	});

	it('production + missing env: never localhost (Vercel split deploy)', () => {
		expect(
			resolvePublicApiBase({
				viteApiUrl: undefined,
				dev: false,
				browser: true
			})
		).toBe('');
		expect(
			resolvePublicApiBase({
				viteApiUrl: undefined,
				dev: false,
				browser: false
			})
		).toBe('');
	});

	it('production + env set in browser: still uses same-origin path', () => {
		expect(
			resolvePublicApiBase({
				viteApiUrl: 'https://swings-api.onrender.com',
				dev: false,
				browser: true
			})
		).toBe('');
	});

	it('production + env set on server: uses API host', () => {
		expect(
			resolvePublicApiBase({
				viteApiUrl: 'https://swings-api.onrender.com',
				dev: false,
				browser: false
			})
		).toBe('https://swings-api.onrender.com');
	});
});
