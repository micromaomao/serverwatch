export default function reltime(time, now) {
  if (time.getTime) {
    time = time.getTime();
  }
  if (now.getTime) {
    now = now.getTime();
  }
  if (time > now) {
    return "???";
  }
  let diff = now - time;
  if (diff < 1000) {
    return `${diff}ms`;
  }
  diff /= 1000;
  if (diff < 60) {
    return `${Math.floor(diff)}s ago`;
  }
  diff /= 60;
  if (diff < 60) {
    return `${Math.floor(diff * 10) / 10}m ago`;
  }
  diff /= 60;
  if (diff < 24) {
    return `${Math.floor(diff * 10) / 10}h ago`;
  }
  diff /= 24;
  return `${Math.floor(diff * 10) / 10}d ago`;
}
