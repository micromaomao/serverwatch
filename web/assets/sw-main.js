const sw_version = 0;

self.addEventListener('install', evt => {
  self.skipWaiting();
});
self.addEventListener('activate', evt => {
  self.clients.claim();
  console.log(`Activated service worker v${sw_version}`);
});
self.addEventListener('message', ({data}) => {
  if (data.update_notification_state) {
    let new_state = data.update_notification_state;
    self.clients.matchAll({includeUncontrolled: true, type: 'window'}).then(clients => {
      for (let c of clients) {
        c.postMessage({update_notification_state: new_state})
      }
    })
    handle_notification_state_change(new_state).then(() => {}, err => {console.error(err)});
  } else if (data.push_test) {
    handle_push_test().then(() => {}, err => {console.error(err)});
  }
});

let server_key = fetch('/application_server_key.base64', {method: 'GET'}).then(x => x.text())

let last_notification_state_change = null;
async function handle_notification_state_change(new_state) {
  last_notification_state_change = new_state;
  let push_manager = self.registration.pushManager;
  let existing_sub = await push_manager.getSubscription();
  if (last_notification_state_change != new_state) { return; }
  let new_has_noti = Object.keys(new_state).length > 0;
  if (existing_sub === null) {
    if (new_has_noti) {
      existing_sub = await push_manager.subscribe({
        userVisibleOnly: true,
        applicationServerKey: await server_key,
      });
      if (last_notification_state_change != new_state) { return; }
    } else {
      return
    }
  }
  await fetch('/api/notification', {method: 'POST', body: JSON.stringify(
    {sub: existing_sub.toJSON(), noti_state: new_state}
  )}).then(res => {
    if (res.status != 200) {
      if (last_notification_state_change != new_state) { return Promise.resolve(); }
      return res.text().then(t => Promise.reject(t));
    } else {
      return Promise.resolve();
    }
  });
  if (last_notification_state_change != new_state) { return; }
  if (!new_has_noti) {
    await existing_sub.unsubscribe();
  }
}

async function handle_push_test() {
  let push_manager = self.registration.pushManager;
  let existing_sub = await push_manager.getSubscription();
  if (!existing_sub) throw new Error("Not subscribed.");
  await fetch('/api/notification/test', {method: 'POST', body: JSON.stringify({
    sub: existing_sub.toJSON()
  })});
}

self.addEventListener('push', evt => {
  let txts = evt.data.text().split('\n');
  let tag = txts[0];
  let timestamp = parseInt(txts[1]);
  let title = txts[2];
  let body = txts.slice(3).join('\n\n');
  evt.waitUntil(self.registration.showNotification(title, {
    body, tag, timestamp, renotify: true, lang: "en"
  }));
})
