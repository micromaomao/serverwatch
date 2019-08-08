<script>
  export let stat;
  let text_mode = false;
  import StatBar from './StatBar.svelte';
</script>

<style>
.quick_statistics {
  color: #444;
  padding: 0 1rem;
  margin-top: 0.5rem;
  margin-bottom: 0.5rem;
  display: flex;
  flex-wrap: wrap;
  flex-direction: row;
  align-items: center;
}
@media only screen and (max-width: 500px) {
  .quick_statistics {
    flex-direction: column;
    align-items: stretch;
  }
}
.quick_statistics > div {
  margin-right: 0.5rem;
  flex-grow: 1;
  display: flex;
  flex-direction: row;
  flex-wrap: nowrap;
  align-items: center;
}
.quick_statistics.text_mode > div {
  align-items: baseline;
}
.bar {
  flex-grow: 1;
  margin-left: 0.3rem;
}
</style>

<div class="quick_statistics{text_mode ? " text_mode" : ""}" on:click={evt => text_mode = !text_mode}>
  <div>Last day: <div class="bar"><StatBar stat={stat.last_day} text_mode={text_mode}/></div></div>
  <div>Last week: <div class="bar"><StatBar stat={stat.last_7_day} text_mode={text_mode}/></div></div>
  <div>Last month: <div class="bar"><StatBar stat={stat.last_month} text_mode={text_mode}/></div></div>
</div>
