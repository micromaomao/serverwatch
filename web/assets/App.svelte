<script>
let check_state = JSON.parse(document.body.dataset.checkState);
import CheckList from './CheckList.svelte';
import { onMount, onDestroy } from 'svelte';
let timeout_handle = null;
let exited = false;
const fetch_delay = 5000;
onMount(() => {
  timeout_handle = setTimeout(function g () {
    timeout_handle = null;
    fetch('/api/status_log', {method: 'GET', credentials: 'omit'}).then(res => {
      if (res.status !== 200) {
        return Promise.reject(res.status);
      } else {
        return res.json();
      }
    }).then(new_state => {
      if (!exited) {
        timeout_handle = setTimeout(g, fetch_delay);
        if (new_state.checks.length != check_state.checks.length) {
          check_state = new_state;
          return;
        }
        for (let i = 0; i < new_state.checks.length; i ++) {
          if (new_state.checks[i].id != check_state.checks[i].id) {
            check_state = new_state;
            return;
          }
        }
        let last_ids = check_state.checks.map(check => (check.log[0] || {id: -1}).id);
        check_state.checks = check_state.checks.map((check, i) => {
          check.log = new_state.checks[i].log.filter(entry => entry.id > last_ids[i]).concat(check.log).slice(0,1000);
          return check;
        });
      }
    }, err => {
      if (!exited) {
        timeout_handle = setTimeout(g, fetch_delay);
      }
    })
  }, fetch_delay)
})
onDestroy(() => {
  if (timeout_handle != null) clearTimeout(timeout_handle);
  exited = true;
})
</script>

<style>
  :global(:root) {
    --color-green: #3CA83C;
    --color-light-green: #C4E4C4;
    --color-orange: #E57248;
    --color-light-orange: #F8DBD1;
    --color-red: #A83C3C;
    --color-light-red: #FDC2C2;
  }

  :global(:root body) {
    padding: 0;
    background-color: white;
    color: #444;
    font-family: sans-serif;
    box-sizing: border-box;
    margin: 0 auto;
  }
</style>

<CheckList checks={check_state.checks} />
