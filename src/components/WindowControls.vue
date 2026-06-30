<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  RemoveOutline as MinIcon,
  ExpandOutline as MaxIcon,
  ContractOutline as RestoreIcon,
  CloseOutline as CloseIcon,
} from "@vicons/ionicons5";
import type { UnlistenFn } from "@tauri-apps/api/event";

const win = getCurrentWindow();
const isMaximized = ref(false);

let unlistenResize: UnlistenFn | null = null;

async function syncMaximized() {
  try {
    isMaximized.value = await win.isMaximized();
  } catch (e) {
    // 非 Tauri 环境（如纯浏览器调试）下忽略
  }
}

async function minimize() {
  try {
    await win.minimize();
  } catch (e) {
    console.error("minimize failed:", e);
  }
}

async function toggleMaximize() {
  try {
    await win.toggleMaximize();
    // toggleMaximize 完成后立即同步一次状态
    await syncMaximized();
  } catch (e) {
    console.error("toggleMaximize failed:", e);
  }
}

async function close() {
  try {
    await win.close();
  } catch (e) {
    console.error("close failed:", e);
  }
}

onMounted(async () => {
  await syncMaximized();
  try {
    unlistenResize = await win.onResized(() => {
      syncMaximized();
    });
  } catch (e) {
    // 非 Tauri 环境忽略
  }
});

onUnmounted(() => {
  unlistenResize?.();
});
</script>

<template>
  <div class="window-controls" data-tauri-drag-region="false">
    <button
      class="wc-btn"
      title="最小化"
      data-tauri-drag-region="false"
      @click="minimize"
    >
      <n-icon :component="MinIcon" size="16" />
    </button>
    <button
      class="wc-btn"
      :title="isMaximized ? '还原' : '最大化'"
      data-tauri-drag-region="false"
      @click="toggleMaximize"
    >
      <n-icon :component="isMaximized ? RestoreIcon : MaxIcon" size="16" />
    </button>
    <button
      class="wc-btn wc-close"
      title="关闭"
      data-tauri-drag-region="false"
      @click="close"
    >
      <n-icon :component="CloseIcon" size="18" />
    </button>
  </div>
</template>

<style scoped>
.window-controls {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-left: 4px;
  /* 允许子元素 overflow 显示 hover 背景 */
  -webkit-app-region: no-drag;
}

.wc-btn {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-base);
  cursor: pointer;
  opacity: 0.7;
  transition: background 0.15s ease, opacity 0.15s ease, color 0.15s ease;
  -webkit-app-region: no-drag;
}

.wc-btn:hover {
  background: var(--hover-bg);
  opacity: 1;
}

.wc-btn:active {
  background: var(--hover-bg-strong);
}

/* 关闭按钮特殊 hover：红色背景 + 白色图标 */
.wc-close:hover {
  background: #e81123;
  color: #fff;
  opacity: 1;
}

.wc-close:active {
  background: #c50f1f;
  color: #fff;
}
</style>
