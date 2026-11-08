// 防误触的时间规则：切页或开关对话框后 500ms 内的点击丢弃，且只认那之后才按下的点击；
// 进入子层后 300ms 内不收 Enter；按住产生的重复 Enter 一律忽略。时间一律用 performance.now()

export const CLICK_GUARD_MS = 500;
export const ENTER_GUARD_MS = 300;

export class InputGuard {
  private layerAt = -Infinity;
  private sublayerAt = -Infinity;
  private pointerDownAt = -Infinity;

  /** 切页、开关对话框 */
  markLayerChange(now: number): void {
    this.layerAt = now;
  }

  /** 进入下一层（子页面、对话框） */
  markSublayerEnter(now: number): void {
    this.sublayerAt = now;
  }

  /** 按下鼠标；按下在切换之前的点击（双击穿透）不算 */
  markPointerDown(now: number): void {
    this.pointerDownAt = now;
  }

  clickAllowed(now: number): boolean {
    return this.pointerDownAt >= this.layerAt && now - this.layerAt >= CLICK_GUARD_MS;
  }

  enterAllowed(now: number, repeat: boolean): boolean {
    return !repeat && now - this.sublayerAt >= ENTER_GUARD_MS;
  }
}

/** 整个页面共用的一份 */
export const inputGuard = new InputGuard();
