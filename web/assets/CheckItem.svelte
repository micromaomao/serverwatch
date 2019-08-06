<script>
  import { yscale } from './animations.js'
  import { createEventDispatcher } from 'svelte';
  import ColorBar from './ColorBar.svelte';
  import LogDisplay from './LogDisplay.svelte';
  export let check;
  export let showing_more;
  const dispatch = createEventDispatcher();

  function handle_show_more () {
    dispatch('togglemore', {});
  }

  $: last_check = (check.log.length > 0 ? check.log[check.log.length - 1] : null);
</script>

<style>
  li.check {
    list-style-type: none;
    padding: 0;
    margin: 0.5rem 0;
    transition: margin 100ms;
    color: #aaa;
    --light-color: #ddd;
  }
  li.check.check-up {
    color: var(--color-green);
    --light-color: var(--color-light-green);
  }
  li.check.check-error {
    color: var(--color-red);
    --light-color: var(--color-light-red);
  }
  li.check.check-warn {
    color: var(--color-orange);
    --light-color: var(--color-light-orange);
  }
  .desc {
    max-width: 100%;
    font-size: 1rem;
    padding: 0.5rem 0;
    border-bottom: 1px solid #fff;
    transition: background-color 300ms, border-bottom 200ms;
    cursor: pointer;
    display: flex;
    flex-direction: row;
    flex-wrap: nowrap;
    white-space: nowrap;
  }
  .desc:hover {
    background-color: #eee;
  }
  .desc > .show-more-btn {
    margin-right: 0.5rem;
    color: #666;
    opacity: 0;
    transition: opacity 300ms, transform 300ms;
    transform: rotate(0deg);
    transform-origin: center center;
  }
  .desc:hover > .show-more-btn {
    opacity: 1;
  }
  .desc > .last-check-info {
    flex-shrink: 1;
    flex-grow: 1;
    text-overflow: ellipsis;
    overflow: hidden;
  }
  .last-state-icon {
    margin-right: 0.5rem;
  }
  li.check.showing-more {
    margin-top: 1rem;
    margin-bottom: 1rem;
  }
  li.check.showing-more .desc {
    border-bottom: 1px solid #aaa;
  }
  li.check.showing-more .desc .show-more-btn {
    transform: rotate(180deg);
  }
  .more {
    padding: 0.5rem 1rem;
  }
  .color-bar {
    margin: -0.5rem -1rem 1rem -1rem;
  }
</style>

<li class={"check check-" + check.last_state + (showing_more ? " showing-more" : "")}>
  <div class="desc" on:click={handle_show_more}>
    <span class="last-state-icon icon-">check_{check.last_state}</span>
    <b>{check.desc}</b>:&nbsp;
    <span class="last-check-info">
      {#if last_check != null}
        {last_check.info}
      {:else}
        pending first check
      {/if}
    </span>
    <span class="show-more-btn icon-">show_more</span>
  </div>
  {#if showing_more}
    <div class="more" transition:yscale="{{duration: 300}}">
      <div class="color-bar">
        <ColorBar log={check.log} />
      </div>
      <LogDisplay log={check.log} />
    </div>
  {/if}
</li>
