<script lang="ts">
	import { onMount, tick } from 'svelte';
	import * as THREE from 'three';
	import type { AnalyticsSummary } from '$lib/api/types';

	interface Props {
		summary: AnalyticsSummary;
	}

	let { summary }: Props = $props();

	let elTime = $state<HTMLDivElement | undefined>();
	let elTop = $state<HTMLDivElement | undefined>();
	let elCtr = $state<HTMLDivElement | undefined>();

	function mountBarChart(container: HTMLDivElement, values: number[], color: number) {
		const width = container.clientWidth || 600;
		const height = 280;
		const scene = new THREE.Scene();
		const camera = new THREE.PerspectiveCamera(45, width / height, 0.1, 1000);
		camera.position.set(0, 6, 14);
		camera.lookAt(0, 2, 0);

		const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
		renderer.setSize(width, height);
		renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
		container.appendChild(renderer.domElement);

		scene.add(new THREE.AmbientLight(0xffffff, 0.5));
		const dir = new THREE.DirectionalLight(0xffffff, 0.9);
		dir.position.set(4, 10, 6);
		scene.add(dir);

		const maxV = Math.max(1, ...values);
		const n = values.length || 1;
		const spacing = 1.1;
		const totalW = (n - 1) * spacing;
		const mat = new THREE.MeshStandardMaterial({ color, metalness: 0.2, roughness: 0.45 });
		const base = new THREE.Mesh(
			new THREE.BoxGeometry(totalW + 2, 0.15, 2),
			new THREE.MeshStandardMaterial({ color: 0x1e293b, metalness: 0.1, roughness: 0.8 })
		);
		base.position.set(0, -0.05, 0);
		scene.add(base);

		const meshes: THREE.Mesh[] = [];
		values.forEach((v, i) => {
			const h = (v / maxV) * 5 + 0.05;
			const geo = new THREE.BoxGeometry(0.7, h, 0.7);
			const mesh = new THREE.Mesh(geo, mat);
			const x = -totalW / 2 + i * spacing;
			mesh.position.set(x, h / 2, 0);
			scene.add(mesh);
			meshes.push(mesh);
		});

		let raf = 0;
		const tick = () => {
			raf = requestAnimationFrame(tick);
			renderer.render(scene, camera);
		};
		tick();

		const ro = new ResizeObserver(() => {
			const w = container.clientWidth || width;
			const h = height;
			camera.aspect = w / h;
			camera.updateProjectionMatrix();
			renderer.setSize(w, h);
		});
		ro.observe(container);

		return () => {
			cancelAnimationFrame(raf);
			ro.disconnect();
			renderer.dispose();
			meshes.forEach((m) => {
				m.geometry.dispose();
				if (Array.isArray(m.material)) m.material.forEach((x) => x.dispose());
				else m.material.dispose();
			});
			base.geometry.dispose();
			(base.material as THREE.Material).dispose();
			container.removeChild(renderer.domElement);
		};
	}

	function mountTrafficChart(
		container: HTMLDivElement,
		pageViews: number[],
		impressions: number[]
	) {
		const width = container.clientWidth || 600;
		const height = 280;
		const scene = new THREE.Scene();
		const camera = new THREE.PerspectiveCamera(45, width / height, 0.1, 1000);
		camera.position.set(0, 6, 14);
		camera.lookAt(0, 2, 0);

		const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
		renderer.setSize(width, height);
		renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
		container.appendChild(renderer.domElement);

		scene.add(new THREE.AmbientLight(0xffffff, 0.5));
		const dir = new THREE.DirectionalLight(0xffffff, 0.9);
		dir.position.set(4, 10, 6);
		scene.add(dir);

		const maxV = Math.max(1, ...pageViews, ...impressions);
		const n = pageViews.length || 1;
		const spacing = 1.1;
		const totalW = (n - 1) * spacing;
		const matPv = new THREE.MeshStandardMaterial({
			color: 0x3b82f6,
			metalness: 0.2,
			roughness: 0.45
		});
		const matImp = new THREE.MeshStandardMaterial({
			color: 0x0fa4af,
			metalness: 0.2,
			roughness: 0.45
		});
		const base = new THREE.Mesh(
			new THREE.BoxGeometry(totalW + 2, 0.15, 2),
			new THREE.MeshStandardMaterial({ color: 0x1e293b, metalness: 0.1, roughness: 0.8 })
		);
		base.position.set(0, -0.05, 0);
		scene.add(base);

		const meshes: THREE.Mesh[] = [];
		for (let i = 0; i < n; i++) {
			const x = -totalW / 2 + i * spacing;
			const pv = pageViews[i] ?? 0;
			const imp = impressions[i] ?? 0;
			const hPv = (pv / maxV) * 5 + 0.05;
			const hImp = (imp / maxV) * 5 + 0.05;
			const gPv = new THREE.Mesh(new THREE.BoxGeometry(0.35, hPv, 0.35), matPv);
			gPv.position.set(x - 0.22, hPv / 2, 0);
			scene.add(gPv);
			meshes.push(gPv);
			const gImp = new THREE.Mesh(new THREE.BoxGeometry(0.35, hImp, 0.35), matImp);
			gImp.position.set(x + 0.22, hImp / 2, 0);
			scene.add(gImp);
			meshes.push(gImp);
		}

		let raf = 0;
		const tick = () => {
			raf = requestAnimationFrame(tick);
			renderer.render(scene, camera);
		};
		tick();

		const ro = new ResizeObserver(() => {
			const w = container.clientWidth || width;
			const h = height;
			camera.aspect = w / h;
			camera.updateProjectionMatrix();
			renderer.setSize(w, h);
		});
		ro.observe(container);

		return () => {
			cancelAnimationFrame(raf);
			ro.disconnect();
			renderer.dispose();
			meshes.forEach((m) => {
				m.geometry.dispose();
			});
			matPv.dispose();
			matImp.dispose();
			base.geometry.dispose();
			(base.material as THREE.Material).dispose();
			container.removeChild(renderer.domElement);
		};
	}

	function mountCtrChart(container: HTMLDivElement, points: AnalyticsSummary['ctr_series']) {
		const width = container.clientWidth || 600;
		const height = 260;
		const scene = new THREE.Scene();
		const camera = new THREE.PerspectiveCamera(45, width / height, 0.1, 1000);
		camera.position.set(0, 5, 12);
		camera.lookAt(0, 1.5, 0);

		const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
		renderer.setSize(width, height);
		renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
		container.appendChild(renderer.domElement);

		scene.add(new THREE.AmbientLight(0xffffff, 0.55));
		const dir = new THREE.DirectionalLight(0xffffff, 0.85);
		dir.position.set(3, 8, 5);
		scene.add(dir);

		const byDay = new Map<string, { imp: number; clk: number }>();
		for (const p of points) {
			const k = p.date;
			const cur = byDay.get(k) ?? { imp: 0, clk: 0 };
			cur.imp += p.impressions;
			cur.clk += p.clicks;
			byDay.set(k, cur);
		}
		const keys = [...byDay.keys()].sort();
		const imps = keys.map((k) => byDay.get(k)!.imp);
		const clks = keys.map((k) => byDay.get(k)!.clk);
		const maxY = Math.max(1, ...imps, ...clks);
		const n = keys.length || 1;
		const spacing = 1.0;
		const totalW = Math.max((n - 1) * spacing, 1);

		const matI = new THREE.MeshStandardMaterial({
			color: 0x0fa4af,
			metalness: 0.25,
			roughness: 0.4
		});
		const matC = new THREE.MeshStandardMaterial({
			color: 0xf59e0b,
			metalness: 0.25,
			roughness: 0.4
		});

		const meshes: THREE.Mesh[] = [];
		keys.forEach((_, i) => {
			const hi = (imps[i]! / maxY) * 4 + 0.05;
			const hc = (clks[i]! / maxY) * 4 + 0.05;
			const x = -totalW / 2 + i * spacing;
			const gI = new THREE.Mesh(new THREE.BoxGeometry(0.35, hi, 0.35), matI);
			gI.position.set(x - 0.22, hi / 2, 0);
			scene.add(gI);
			meshes.push(gI);
			const gC = new THREE.Mesh(new THREE.BoxGeometry(0.35, hc, 0.35), matC);
			gC.position.set(x + 0.22, hc / 2, 0);
			scene.add(gC);
			meshes.push(gC);
		});

		let raf = 0;
		const tick = () => {
			raf = requestAnimationFrame(tick);
			renderer.render(scene, camera);
		};
		tick();

		const ro = new ResizeObserver(() => {
			const w = container.clientWidth || width;
			camera.aspect = w / height;
			camera.updateProjectionMatrix();
			renderer.setSize(w, height);
		});
		ro.observe(container);

		return () => {
			cancelAnimationFrame(raf);
			ro.disconnect();
			renderer.dispose();
			meshes.forEach((m) => {
				m.geometry.dispose();
			});
			matI.dispose();
			matC.dispose();
			container.removeChild(renderer.domElement);
		};
	}

	onMount(() => {
		const cleanups: (() => void)[] = [];
		let cancelled = false;

		void tick().then(() => {
			if (cancelled) return;
			if (elTime && summary.time_series.length) {
				const pvs = summary.time_series.map((t) => t.page_views);
				const imps = summary.time_series.map((t) => t.impressions);
				const c = mountTrafficChart(elTime, pvs, imps);
				if (cancelled) c();
				else cleanups.push(c);
			}
			if (cancelled) return;
			if (elTop && summary.top_pages.length) {
				const values = summary.top_pages.map((p) => p.views);
				const c = mountBarChart(elTop, values, 0x8b5cf6);
				if (cancelled) c();
				else cleanups.push(c);
			}
			if (cancelled) return;
			if (elCtr && summary.ctr_series.length) {
				const c = mountCtrChart(elCtr, summary.ctr_series);
				if (cancelled) c();
				else cleanups.push(c);
			}
		});

		return () => {
			cancelled = true;
			cleanups.forEach((c) => c());
		};
	});
</script>

<div class="dash3d">
	{#if summary.time_series.length}
		<section class="dash3d__block">
			<h3 class="dash3d__title">Page views & impressions over time</h3>
			<p class="dash3d__hint">Blue = page views, teal = impression events (3D bars)</p>
			<div bind:this={elTime} class="dash3d__canvas"></div>
			<table class="dash3d__table">
				<thead>
					<tr>
						<th>Date</th>
						<th>Views</th>
						<th>Impr.</th>
						<th>Sessions</th>
					</tr>
				</thead>
				<tbody>
					{#each summary.time_series as row}
						<tr>
							<td>{row.date}</td>
							<td>{row.page_views}</td>
							<td>{row.impressions}</td>
							<td>{row.unique_sessions}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</section>
	{/if}

	{#if summary.top_pages.length}
		<section class="dash3d__block">
			<h3 class="dash3d__title">Top pages</h3>
			<p class="dash3d__hint">Most viewed paths</p>
			<div bind:this={elTop} class="dash3d__canvas"></div>
			<table class="dash3d__table">
				<thead>
					<tr>
						<th>Path</th>
						<th>Views</th>
					</tr>
				</thead>
				<tbody>
					{#each summary.top_pages as row}
						<tr>
							<td class="dash3d__path">{row.path}</td>
							<td>{row.views}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</section>
	{/if}

	{#if summary.ctr_series.length}
		<section class="dash3d__block">
			<h3 class="dash3d__title">CTR (impressions vs clicks)</h3>
			<p class="dash3d__hint">Teal = impressions, amber = clicks (per day)</p>
			<div bind:this={elCtr} class="dash3d__canvas"></div>
			<table class="dash3d__table">
				<thead>
					<tr>
						<th>Date</th>
						<th>CTA</th>
						<th>Impr.</th>
						<th>Clicks</th>
						<th>CTR</th>
					</tr>
				</thead>
				<tbody>
					{#each summary.ctr_series as row}
						<tr>
							<td>{row.date}</td>
							<td>{row.cta_id}</td>
							<td>{row.impressions}</td>
							<td>{row.clicks}</td>
							<td>{(row.ctr * 100).toFixed(2)}%</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</section>
	{:else}
		<section class="dash3d__block dash3d__block--empty">
			<p>
				No CTR data yet. Track <code>impression</code> / <code>click</code> events with
				<code>metadata.cta_id</code> from your CTAs.
			</p>
		</section>
	{/if}
</div>

<style>
	.dash3d {
		display: flex;
		flex-direction: column;
		gap: 2rem;
	}

	.dash3d__block {
		border-radius: var(--radius-lg, 0.75rem);
		border: 1px solid rgba(255, 255, 255, 0.08);
		background: rgba(0, 0, 0, 0.25);
		padding: 1.25rem;
	}

	.dash3d__block--empty {
		color: var(--color-grey-400, #9ca3af);
		font-size: var(--fs-sm, 0.875rem);
	}

	.dash3d__block--empty code {
		color: var(--color-teal-light, #5eead4);
		font-size: 0.8em;
	}

	.dash3d__title {
		margin: 0 0 0.25rem;
		font-size: 1.1rem;
		font-weight: 600;
		color: var(--color-white, #fff);
	}

	.dash3d__hint {
		margin: 0 0 1rem;
		font-size: var(--fs-xs, 0.75rem);
		color: var(--color-grey-500, #6b7280);
	}

	.dash3d__canvas {
		width: 100%;
		min-height: 280px;
		margin-bottom: 1rem;
		border-radius: var(--radius-md, 0.5rem);
		overflow: hidden;
		background: linear-gradient(180deg, rgba(15, 23, 42, 0.9), rgba(15, 23, 42, 0.4));
	}

	.dash3d__table {
		width: 100%;
		border-collapse: collapse;
		font-size: var(--fs-xs, 0.75rem);
	}

	.dash3d__table th,
	.dash3d__table td {
		padding: 0.4rem 0.5rem;
		text-align: left;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}

	.dash3d__table th {
		color: var(--color-grey-500, #6b7280);
		font-weight: 500;
	}

	.dash3d__path {
		max-width: 18rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-family: var(--font-ui, ui-monospace, monospace);
	}
</style>
