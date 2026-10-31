<!--
  Embeds the Cookie D'oh! wasm build in a SvelteKit page (Svelte 5).

  Copy the release build into your app's static folder first:
    task web:export DEST=../my-site/static/cookie-doh

  Then:
    <CookieDoh onexit={() => goto('/games')} ongameover={(r) => saveScore(r.score)} />

  The game binds to its <canvas> once and has no teardown, so the canvas and the wasm module
  live for the whole page session. Unmounting this component only detaches the canvas (the
  game pauses and goes silent while detached); mounting it again re-attaches the same canvas.

  `cracktro` and `gfx` are read once, when the game first starts.
-->
<script module lang="ts">
	export interface GameOver {
		score: number;
		level: number;
		hiscore: number;
		/** The score beat the stored hi-score. */
		record: boolean;
	}

	let canvas: HTMLCanvasElement | null = null;
	let started: Promise<void> | null = null;

	function getCanvas(cracktro: boolean, gfx: string | undefined): HTMLCanvasElement {
		if (!canvas) {
			canvas = document.createElement('canvas');
			canvas.id = 'cookie-doh';
			canvas.tabIndex = 0;
			canvas.setAttribute('aria-label', "Cookie D'oh! arcade game");
			if (cracktro) canvas.dataset.cracktro = '';
			if (gfx) canvas.dataset.gfx = gfx;
		}
		return canvas;
	}

	function start(base: string): Promise<void> {
		started ??= import(/* @vite-ignore */ `${base}/cookie-doh.js`).then((m) => m.default());
		return started;
	}
</script>

<script lang="ts">
	interface Props {
		/** URL path the build was exported to (no trailing slash). */
		base?: string;
		/** Focus the game on mount so the keyboard works right away. */
		autofocus?: boolean;
		/** Show the Cookies & Crackers cracktro first. */
		cracktro?: boolean;
		/** Start in a historical graphics mode: 'ega' | 'c64' | 'cga' | 'hercules'. */
		gfx?: 'vga' | 'ega' | 'c64' | 'cga' | 'hercules';
		/** The player pressed Esc on the title screen; the game has gone quiet. */
		onexit?: () => void;
		/** A game started. */
		onstart?: () => void;
		/** A game ended. */
		ongameover?: (result: GameOver) => void;
	}

	let {
		base = '/cookie-doh',
		autofocus = false,
		cracktro = false,
		gfx,
		onexit,
		onstart,
		ongameover
	}: Props = $props();

	let error = $state<string | null>(null);

	/** Attach the shared canvas to `slot`, start the game once, detach on unmount. */
	function mount(slot: HTMLElement) {
		const c = getCanvas(cracktro, gfx);
		const exit = () => onexit?.();
		const begin = () => onstart?.();
		const over = (e: Event) => ongameover?.((e as CustomEvent<GameOver>).detail);
		c.addEventListener('cookie-doh:exit', exit);
		c.addEventListener('cookie-doh:start', begin);
		c.addEventListener('cookie-doh:gameover', over);
		slot.appendChild(c);
		if (autofocus) c.focus();
		// The canvas must be in the document before the game starts, as it looks it up by id.
		start(base).catch((e) => {
			// winit throws a control-flow exception once the event loop is running; that is expected.
			if (!String(e).includes('Using exceptions for control flow')) {
				error = String(e);
				started = null;
			}
		});
		return () => {
			c.removeEventListener('cookie-doh:exit', exit);
			c.removeEventListener('cookie-doh:start', begin);
			c.removeEventListener('cookie-doh:gameover', over);
			c.remove();
		};
	}
</script>

<div class="cookie-doh">
	<div class="slot" {@attach mount}></div>
	{#if error}
		<p class="error">Could not start the game: {error}</p>
	{/if}
</div>

<style>
	.cookie-doh {
		position: relative;
		width: 100%;
		max-width: 1280px;
		/* 320x200: the shape of the game's screen; it letterboxes inside. */
		aspect-ratio: 16 / 10;
		background: #000;
	}
	.slot {
		position: absolute;
		inset: 0;
	}
	.slot :global(canvas#cookie-doh) {
		display: block;
		/* winit sets an inline size; the container decides instead. */
		width: 100% !important;
		height: 100% !important;
		outline: none;
		touch-action: none;
		user-select: none;
		-webkit-user-select: none;
		-webkit-touch-callout: none;
		image-rendering: pixelated;
	}
	.error {
		position: absolute;
		inset: 0;
		margin: 0;
		display: grid;
		place-items: center;
		color: #f06a6a;
		font: 14px system-ui, sans-serif;
	}
</style>
