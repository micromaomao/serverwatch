<script>
let check_state = JSON.parse(document.body.dataset.checkState);
import CheckList from './CheckList.svelte';
import { onMount, onDestroy } from 'svelte';
import { notification_state, canNotify, ready as notification_ready, set_notification_state, show_sample as notification_show_sample } from './NotificationManager.js';
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
        check_state.checks = check_state.checks.map((old_check, i) => {
          return Object.assign(new_state.checks[i], {
            log: new_state.checks[i].log.filter(entry => entry.id > last_ids[i]).concat(old_check.log).slice(0,1000)
          });
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
$: all_system_ok = check_state.checks.every(c => c.last_state == "up");
$: some_system_warn = check_state.checks.every(c => c.last_state == "up" || c.last_state == "warn");
$: some_system_down = check_state.checks.find(c => c.last_state == "error");

$: num_notification_triggers = Object.keys($notification_state).length;
let set_notification_mode = false;

function toggle_notification_mode () {
  if (set_notification_mode) {
    set_notification_mode = false;
    return;
  }
  if (!$canNotify) return;
  notification_ready().then(() => {
    set_notification_mode = true;
  })
}

function disable_all_notifications () {
  set_notification_state({})
}
function enable_all_errors () {
  let ns = $notification_state;
  let old_alls = [];
  for (let c of Object.keys(ns)) {
    if (ns[c] && ns[c].notify_warn) {
      old_alls.push(c);
    }
  }
  for (let c of check_state.checks.map(x => x.id)) {
    ns[c] = {notify_warn: false};
  }
  for (let c of old_alls) {
    ns[c] = {notify_warn: true};
  }
  set_notification_state(ns);
}
function enable_all () {
  let ns = {};
  for (let c of check_state.checks.map(x => x.id)) {
    ns[c] = {notify_warn: true};
  }
  set_notification_state(ns);
}
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

  .header {
    font-size: 0.9rem;
    padding: 0;
    display: flex;
    flex-direction: row;
    flex-wrap: wrap;
    align-items: baseline;
  }

  .header > .general-state {
    padding: 0.5rem 1.2rem;
    margin: 0;
  }

  .all-ok {
    background-color: var(--color-light-green);
    color: var(--color-green);
  }
  .some-warns {
    background-color: var(--color-light-orange);
    color: var(--color-orange);
  }
  .some-down {
    background-color: var(--color-light-red);
    color: var(--color-red);
  }

  .noti-btn {
    padding: 0.5rem 1rem;
    margin: 0;
    cursor: pointer;
    transition: background-color 200ms;
  }

  .noti-btn:hover {
    background-color: #eee;
  }
</style>

<div class="header">
  {#if set_notification_mode}
    <span class="noti-btn" on:click={toggle_notification_mode}>
      back
    </span>
    &nbsp;
    {num_notification_triggers} notification{num_notification_triggers > 1 ? 's' : ''} set
    &nbsp;
    <span class="noti-btn" on:click={disable_all_notifications}>
      <span class="icon-">notifications_off</span> disable all
    </span>
    <span class="noti-btn" on:click={enable_all_errors}>
      <span class="icon-">notifications</span> enable all (only errors)
    </span>
    <span class="noti-btn" on:click={enable_all}>
      <span class="icon-">notifications</span> enable all
    </span>
  {:else}
    {#if all_system_ok}
      <span class="general-state all-ok">All system up</span>
    {:else if some_system_warn}
      <span class="general-state some-warns">Some system warns</span>
    {:else if some_system_down}
      <span class="general-state some-down">Some system down</span>
    {/if}
    <span class="noti-btn" on:click={toggle_notification_mode}>
      {#if !$canNotify}
        <span class="icon-">notifications_off</span> Notification unavailable
      {:else if num_notification_triggers == 0}
        <span class="icon-">notifications_off</span> No notifications set
      {:else}
        <span class="icon-">notifications</span> {num_notification_triggers} notification{num_notification_triggers > 1 ? 's' : ''} set
      {/if}
    </span>
  {/if}
</div>
<CheckList checks={check_state.checks} editing_notifications={set_notification_mode} />
