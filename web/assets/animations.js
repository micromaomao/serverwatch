import { cubicOut } from 'svelte/easing'
import { parse } from 'terser';
export function yscale(node, {
  delay = 0,
  duration = 500,
  easing: easing = cubicOut,
}) {
  let style = window.getComputedStyle(node);
  let full_height = parseInt(style.height);
  let pad_top = parseInt(style.paddingTop);
  let pad_bottom = parseInt(style.paddingBottom);

  return {
    delay, duration, easing,
    css: t => `height: ${t * full_height}px; padding-top: ${t * pad_top}px; padding-bottom: ${t * pad_bottom}px; overflow: hidden`,
  }
}
