// 不打断的提示（如开始游戏被拒）：舞台左下角一行小字，几秒后淡出；新的一条替换旧的

const SHOW_MS = 3000;

let el: HTMLElement | null = null;
let timer = 0;

export function showToast(text: string): void {
  if (!text) return;
  const host = document.getElementById('stage') ?? document.body;
  if (!el) {
    el = document.createElement('p');
    el.className = 'toast';
    el.setAttribute('role', 'status');
  }
  if (el.parentElement !== host) host.append(el);
  el.textContent = text;
  el.classList.add('on');
  clearTimeout(timer);
  timer = window.setTimeout(() => el?.classList.remove('on'), SHOW_MS);
}

/** 接口被拒：记日志并提示原因（Rust 给的中文字符串） */
export function toastError(e: unknown): void {
  console.error('接口调用被拒', e);
  showToast(e instanceof Error ? e.message : String(e));
}
