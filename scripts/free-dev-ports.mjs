#!/usr/bin/env node
/**
 * Frees default dev ports before `pnpm dev:all` so Vite + the API can bind.
 * macOS/Linux: uses `lsof`. Other platforms: no-op (start may fail if busy).
 */
import { execSync } from 'node:child_process';
import process from 'node:process';

const ports = [5173, 5174, 5175, 5176, 3001];

if (process.platform === 'win32') {
	process.exit(0);
}

function pidsOnPort(port) {
	try {
		const out = execSync(`lsof -ti tcp:${port}`, {
			encoding: 'utf8',
			stdio: ['ignore', 'pipe', 'ignore']
		}).trim();
		if (!out) return [];
		return [...new Set(out.split(/\s+/).filter(Boolean))];
	} catch {
		return [];
	}
}

for (const port of ports) {
	const pids = pidsOnPort(port);
	for (const pid of pids) {
		const n = Number(pid);
		if (!Number.isFinite(n) || n === process.pid) continue;
		try {
			process.kill(n, 'SIGKILL');
			console.warn(`[free-dev-ports] killed pid ${n} on port ${port}`);
		} catch {
			/* ignore */
		}
	}
}
