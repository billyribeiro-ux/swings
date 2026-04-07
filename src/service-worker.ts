/// <reference lib="webworker" />
/// <reference types="@sveltejs/kit" />
import { build, files, version } from '$service-worker';

declare const self: ServiceWorkerGlobalScope;

const CACHE = `swings-${version}`;

const ASSETS = [...build, ...files];

// ── Install: cache app shell ──────────────────────────────────────────
self.addEventListener('install', (e) => {
	e.waitUntil(
		caches
			.open(CACHE)
			.then((c) => c.addAll(ASSETS))
			.then(() => self.skipWaiting())
	);
});

// ── Activate: purge old caches ────────────────────────────────────────
self.addEventListener('activate', (e) => {
	e.waitUntil(
		caches.keys().then(async (keys) => {
			for (const key of keys) {
				if (key !== CACHE) await caches.delete(key);
			}
			self.clients.claim();
		})
	);
});

// ── Offline draft queue (IndexedDB) ───────────────────────────────────

const DB_NAME = 'swings-offline';
const STORE = 'draft-queue';

function openDb(): Promise<IDBDatabase> {
	return new Promise((resolve, reject) => {
		const req = indexedDB.open(DB_NAME, 1);
		req.onupgradeneeded = () => req.result.createObjectStore(STORE, { autoIncrement: true });
		req.onsuccess = () => resolve(req.result);
		req.onerror = () => reject(req.error);
	});
}

async function enqueue(request: Request): Promise<void> {
	const db = await openDb();
	const body = await request.text();
	const entry = { url: request.url, method: request.method, body, timestamp: Date.now() };
	return new Promise((resolve, reject) => {
		const tx = db.transaction(STORE, 'readwrite');
		tx.objectStore(STORE).add(entry);
		tx.oncomplete = () => resolve();
		tx.onerror = () => reject(tx.error);
	});
}

async function flushQueue(): Promise<void> {
	const db = await openDb();
	const tx = db.transaction(STORE, 'readwrite');
	const store = tx.objectStore(STORE);
	const all: { key: IDBValidKey; entry: { url: string; method: string; body: string } }[] = [];

	await new Promise<void>((resolve) => {
		store.openCursor().onsuccess = function () {
			const cursor = this.result as IDBCursorWithValue | null;
			if (cursor) {
				all.push({ key: cursor.key, entry: cursor.value });
				cursor.continue();
			} else {
				resolve();
			}
		};
	});

	for (const { key, entry } of all) {
		try {
			await fetch(entry.url, {
				method: entry.method,
				headers: { 'Content-Type': 'application/json' },
				body: entry.body
			});
			store.delete(key);
		} catch {
			break;
		}
	}
}

// ── Fetch: intercept autosave PUTs when offline ───────────────────────

const AUTOSAVE_RE = /\/api\/admin\/blog\/posts\/[a-f0-9-]+$/;

self.addEventListener('fetch', (e) => {
	const { request } = e;
	const url = new URL(request.url);

	if (request.method === 'PUT' && AUTOSAVE_RE.test(url.pathname)) {
		e.respondWith(
			fetch(request.clone()).catch(async () => {
				await enqueue(request.clone());
				return new Response(JSON.stringify({ queued: true }), {
					status: 202,
					headers: { 'Content-Type': 'application/json' }
				});
			})
		);
		return;
	}

	if (request.method !== 'GET') return;

	const isAsset = ASSETS.includes(url.pathname) || url.pathname.startsWith('/immutable/');

	e.respondWith(
		isAsset
			? caches.match(request).then((cached) => cached ?? fetch(request))
			: fetch(request).catch(() => caches.match(request) as Promise<Response>)
	);
});

// ── Sync queued drafts when back online ──────────────────────────────

self.addEventListener('sync', (e) => {
	if ((e as SyncEvent).tag === 'draft-sync') {
		(e as SyncEvent).waitUntil(flushQueue());
	}
});

self.addEventListener('message', (e) => {
	if (e.data?.type === 'FLUSH_QUEUE') {
		flushQueue();
	}
});

// TypeScript helper for the sync event
interface SyncEvent extends ExtendableEvent {
	readonly tag: string;
}
