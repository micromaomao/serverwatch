<script>
  import { now } from './store.js'
  import reltime from './reltime.js'
  export let log;
  function apply_filter(show_up, show_warn, show_err, log) {
    return log.filter(l => {
      if (l.state == "up" && show_up) return true;
      if (l.state == "warn" && show_warn) return true;
      if (l.state == "error" && show_err) return true;
      return false;
    });
  }
  let show_up = false;
  let show_warn = true;
  let show_err = true;
  $: log_showing = apply_filter(show_up, show_warn, show_err, log).map(l => Object.assign({}, l, {time: new Date(l.time)}));
  let time_utc = false;
</script>

<style>
  ul {
    max-height: 50vh;
    overflow: auto;
    overflow-y: scroll;
    -webkit-overflow-scrolling: touch;
    margin-bottom: 0.5rem;
  }
  .l-up {
    color: var(--color-green);
  }
  .l-warn {
    color: var(--color-orange);
  }
  .l-error {
    color: var(--color-red);
  }
  li {
    margin: 0.5rem 0;
    padding: 0;
    list-style-type: none;
  }
  .filter-control {
    color: #444;
    border: solid 1px #aaa;
    padding: 0.5rem;
    margin-top: 0.5rem;
    margin-bottom: 1rem;
  }
  .filter-control .filter-icon {
    font-size: 1.3em;
  }
  .filter-control > label.mono {
    font-family: monospace;
    font-size: 1rem;
    margin-right: 0.5rem;
  }
  .filter-control > label > input {
    margin-right: 0.2rem;
  }
  .filter-control > .sep {
    margin: 0 0.5rem;
  }
</style>

<div class="filter-control">
  <span class="icon- filter-icon">filter</span>&nbsp;
  <label class="mono"><input type="checkbox" bind:checked={show_up} />UP</label>
  <label class="mono"><input type="checkbox" bind:checked={show_warn} />WARN</label>
  <label class="mono"><input type="checkbox" bind:checked={show_err} />ERR</label>
  <span class="sep" />
  <label><input type="checkbox" bind:checked={time_utc} />show time in UTC</label>
</div>
{#if log_showing.length > 0}
  <ul>
    {#each log_showing as l (l.id)}
      <li class="l-{l.state}">{time_utc ? l.time.toISOString() : l.time.toLocaleTimeString()} ({reltime(l.time, $now)}): {l.state.toUpperCase()} {l.info}</li>
    {/each}
  </ul>
{:else}
  <span style="color: #444">(nothing)</span>
{/if}
