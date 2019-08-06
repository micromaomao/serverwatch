<script>
  export let checks;
  import CheckItem from './CheckItem.svelte';
  let showing_more_desc = null;

  function handle_toggle_more(desc) {
    if (showing_more_desc == desc) {
      showing_more_desc = null;
    } else {
      showing_more_desc = desc;
    }
  }

</script>

<style>
  ul {
    margin: 1rem;
    padding: 0;
  }
</style>

<ul>
  {#each checks as check}
    <CheckItem check={check} showing_more={showing_more_desc != null && check.desc == showing_more_desc}
      on:togglemore="{e => handle_toggle_more(check.desc)}" />
  {/each}
</ul>
