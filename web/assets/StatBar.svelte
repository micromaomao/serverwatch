<script>
	export let stat;
	$: st = [stat.num_up, stat.num_warn, stat.num_error];
	$: total = st.reduce((a, b) => a + b);
	$: precentages = st.map(x => x / total * 100);
	export let text_mode;
</script>

<style>
	.bar {
		height: 1em;
		background-color: white;
		display: flex;
		flex-wrap: nowrap;
		flex-direction: row;
		padding: 0;
		width: 100%;
		align-items: stretch;
	}
	.bar > div {
		flex-shrink: 0;
		flex-grow: 0;
		height: 100%;
	}
	.up {
		background-color: var(--color-green);
	}
	.uptext {
		color: var(--color-green);
	}
	.warn {
		background-color: var(--color-orange);
	}
	.warntext {
		color: var(--color-orange);
	}
	.error {
		background-color: var(--color-red);
	}
	.errortext {
		color: var(--color-red);
	}
</style>

<div class="bar">
	{#if !text_mode}
		<div class="up" style="width: {precentages[0]}%;" />
		<div class="warn" style="width: {precentages[1]}%;" />
		<div class="error" style="width: {precentages[2]}%;" />
	{:else}
		<div class="uptext">{st[0]}</div>/
		<div class="warntext">{st[1]}</div>/
		<div class="errortext">{st[2]}</div>
	{/if}
</div>
