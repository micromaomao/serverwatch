import { readable } from 'svelte/store';

export const now = readable(new Date(), function (set) {
  let handle = requestAnimationFrame(function ani () {
    handle = requestAnimationFrame(ani);
    set(new Date());
  });

  return () => cancelAnimationFrame(handle);
})
