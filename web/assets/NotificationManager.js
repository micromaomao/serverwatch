let currentConfig = [];
import { readable, writable } from 'svelte/store';

export let canNotify = writable(false);
let _canNotify = false;
canNotify.subscribe(val => _canNotify = val);
let _notification_state = null;
export let notification_state = writable({});
/* notification_state is a Object mapping string of check_id to:
     {
       notify_warn: bool
       // error is always notified.
     }
*/
notification_state.subscribe(val => _notification_state = val);

if (!('serviceWorker' in window.navigator)) {
  console.log("Service worker not supported by browser.");
} else if (!window.Notification) {
  console.log("Notification not supported by browser.");
} else if (!window.PushManager) {
  console.log("PushManager not supported by browser.");
} else {
  init();
}

function init() {
  navigator.serviceWorker.register('/sw.js', {
    scope: '/',
  }).then(swr => {
    let state_from_local_storage = JSON.parse(localStorage.getItem("notification_state"));
    if (state_from_local_storage) {
      notification_state.set(state_from_local_storage);
    }
    canNotify.set(true);
    navigator.serviceWorker.addEventListener('message', ({data}) => {
      if (data.update_notification_state) {
        let new_state = data.update_notification_state;
        notification_state.set(new_state);
      }
    })
  });
}

export async function set_notification_state(new_state) {
  if (!_canNotify) throw new Error("Notification module not initialized yet (or can't be initialized)");
  let reg = await navigator.serviceWorker.ready;
  if (reg.active) {
    reg.active.postMessage({update_notification_state: new_state});
    localStorage.setItem('notification_state', JSON.stringify(new_state));
  } else {
    return Promise.reject(new Error("Service worker not active, for some reason\u2026"));
  }
}

export function ready() {
  if (Notification.permission === "granted") return Promise.resolve();
  return Notification.requestPermission().then(result => {
    if (result === "granted") {
      return Promise.resolve();
    } else {
      return Promise.reject();
    }
  })
}

export async function push_test() {
  if (!_canNotify) throw new Error("Notification module not initialized yet (or can't be initialized)");
  let reg = await navigator.serviceWorker.ready;
  if (reg.active) {
    reg.active.postMessage({push_test: true});
  } else {
    return Promise.reject(new Error("Service worker not active, for some reason\u2026"));
  }
}
