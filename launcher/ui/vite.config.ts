import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';

const page = (name: string) => fileURLToPath(new URL(name, import.meta.url));

// NRGA_PROBE=1：探测构建，额外打包组件测试页 probe.html（test.ps1 设置）
const probe = process.env.NRGA_PROBE === '1';

export default defineConfig({
  // 相对路径，供 Tauri 内嵌资源加载
  base: './',
  build: {
    outDir: 'dist',
    // 贴图和字体一律输出成文件，不内联成 base64
    assetsInlineLimit: 0,
    rollupOptions: probe
      ? { input: { index: page('index.html'), probe: page('probe.html') } }
      : {},
  },
});
