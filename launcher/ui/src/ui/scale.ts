// 等比缩放（草图 v4 的 fit()）：设计稿固定 1200×720，整个舞台按窗口等比缩放并居中

export const DESIGN_W = 1200;
export const DESIGN_H = 720;

export function fit(stage: HTMLElement, w = innerWidth, h = innerHeight): void {
  const s = Math.min(w / DESIGN_W, h / DESIGN_H);
  // 窗口还没有尺寸（隐藏中）时不动
  if (!(s > 0) || !Number.isFinite(s)) return;
  stage.style.transform = `translate(${(w - DESIGN_W * s) / 2}px,${(h - DESIGN_H * s) / 2}px) scale(${s})`;
}

/** 立即缩放一次并跟随窗口尺寸变化，返回取消函数 */
export function installScale(stage: HTMLElement): () => void {
  const onResize = () => fit(stage);
  onResize();
  addEventListener('resize', onResize);
  return () => removeEventListener('resize', onResize);
}
