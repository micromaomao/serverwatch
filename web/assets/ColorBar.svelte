<script>
  import { now } from './store.js'
  export let log;
  function to_bars(log) {
    log = log.slice().sort((a, b) => Math.sign(a.time - b.time));
    let bars = [];
    let last_bar = null;
    for (let l of log) {
      if (last_bar == null) {
        last_bar = {
          state: l.state,
          time_start: l.time,
          time_end: l.time,
        };
        bars.push(last_bar);
      } else {
        last_bar.time_end = l.time;
        if (last_bar.state != l.state) {
          last_bar = {
            state: l.state,
            time_start: l.time,
            time_end: l.time,
          };
          bars.push(last_bar);
        }
      }
    }
    return bars;
  }
  $: bars = to_bars(log);

  const scale_factor = 100/(1000*60*30);
</script>

<style>
  .color-bar {
    height: 4px;
    width: 100%;
    display: block;
    padding: 0;
    margin: 0;
    background-color: transparent;
    position: relative;
    overflow: hidden;
  }
  .bar {
    position: absolute;
    top: 0;
    bottom: 2px;
  }
  .bar_up {
    background-color: var(--color-green);
  }
  .bar_warn {
    background-color: var(--color-orange);
    bottom: 0;
  }
  .bar_error {
    background-color: var(--color-red);
    bottom: 0;
  }
</style>

<div class="color-bar">
  {#each bars as bar}
    <div class="bar bar_{bar.state}" style="left: {($now - bar.time_end) * scale_factor}%; width: {(bar.time_end - bar.time_start) * scale_factor}%;" />
  {/each}
</div>
