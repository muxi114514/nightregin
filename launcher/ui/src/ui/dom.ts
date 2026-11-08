// 建节点的小工具
export function el<K extends keyof HTMLElementTagNameMap>(tag: K, className = ''): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (className) e.className = className;
  return e;
}

export const asset = (name: string) => `${import.meta.env.BASE_URL}assets/${name}`;
