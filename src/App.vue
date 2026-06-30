<script setup lang="ts">
import { darkTheme, type GlobalThemeOverrides } from "naive-ui";
import { computed, onMounted, watch } from "vue";
import { useSettingsStore } from "@/stores/settings";

const settings = useSettingsStore();

// 主题覆盖：根据 isDark 返回不同配色，让 naive-ui 的输入框/选择框/下拉菜单
// 与应用的玻璃态主题协调，避免默认深色背景在 #0a0a0c 上显得突兀。
const themeOverrides = computed<GlobalThemeOverrides>(() => {
  const dark = settings.isDark;
  return {
    common: {
      primaryColor: "#6366f1",
      primaryColorHover: "#7c7ff5",
      primaryColorPressed: "#5258e8",
      borderRadius: "8px",
      // 输入框 / 选择框触发器背景：极淡白色，与 panel 一致
      inputColor: dark ? "rgba(255, 255, 255, 0.04)" : "rgba(255, 255, 255, 0.7)",
      inputColorDisabled: dark ? "rgba(255, 255, 255, 0.02)" : "rgba(245, 245, 247, 0.6)",
      // 边框：弱化，hover/focus 时主色高亮
      inputBorder: dark ? "rgba(255, 255, 255, 0.08)" : "rgba(0, 0, 0, 0.1)",
      inputBorderHover: "rgba(99, 102, 241, 0.4)",
      inputBorderFocus: "rgba(99, 102, 241, 0.55)",
      // 占位符 / 文字
      inputPlaceholderColor: dark ? "rgba(255, 255, 255, 0.3)" : "rgba(0, 0, 0, 0.35)",
      textColor: dark ? "rgba(255, 255, 255, 0.9)" : "rgba(17, 17, 20, 0.88)",
      // 弹出层（下拉菜单 / popover / tooltip）背景：与应用深色协调
      popoverColor: dark ? "rgba(26, 26, 30, 0.98)" : "#ffffff",
      selectMenuColor: dark ? "rgba(26, 26, 30, 0.98)" : "#ffffff",
      // 弹出层边框/阴影微调
      popoverBorderColor: dark ? "rgba(255, 255, 255, 0.08)" : "rgba(0, 0, 0, 0.08)",
      // 卡片/面板（部分组件用 cardColor）
      cardColor: dark ? "rgba(255, 255, 255, 0.025)" : "rgba(255, 255, 255, 0.75)",
      // 模态/遮罩
      modalColor: dark ? "rgba(24, 24, 27, 0.92)" : "rgba(255, 255, 255, 0.95)",
    },
    // n-select 专项：让菜单选项 hover/选中色与主色呼应
    Select: {
      menuColor: dark ? "rgba(26, 26, 30, 0.98)" : "#ffffff",
      optionColorActive: dark ? "rgba(99, 102, 241, 0.18)" : "rgba(99, 102, 241, 0.12)",
      optionColorPending: dark ? "rgba(255, 255, 255, 0.06)" : "rgba(0, 0, 0, 0.04)",
      optionColorActivePending: dark ? "rgba(99, 102, 241, 0.24)" : "rgba(99, 102, 241, 0.18)",
      peers: {
        InternalSelection: {
          color: dark ? "rgba(255, 255, 255, 0.04)" : "rgba(255, 255, 255, 0.7)",
          colorActive: dark ? "rgba(255, 255, 255, 0.06)" : "rgba(255, 255, 255, 0.9)",
          border: dark ? "rgba(255, 255, 255, 0.08)" : "rgba(0, 0, 0, 0.1)",
          borderHover: "rgba(99, 102, 241, 0.4)",
          borderFocus: "rgba(99, 102, 241, 0.55)",
          borderActive: "rgba(99, 102, 241, 0.55)",
        },
      },
    },
    // n-input 专项
    Input: {
      color: dark ? "rgba(255, 255, 255, 0.04)" : "rgba(255, 255, 255, 0.7)",
      colorFocus: dark ? "rgba(255, 255, 255, 0.06)" : "rgba(255, 255, 255, 0.9)",
      border: dark ? "rgba(255, 255, 255, 0.08)" : "rgba(0, 0, 0, 0.1)",
      borderHover: "rgba(99, 102, 241, 0.4)",
      borderFocus: "rgba(99, 102, 241, 0.55)",
    },
    // n-drawer 内部背景对齐
    Drawer: {
      bodyColor: dark ? "rgba(24, 24, 27, 0.85)" : "rgba(245, 245, 247, 0.92)",
      headerBorder: dark ? "rgba(255, 255, 255, 0.06)" : "rgba(0, 0, 0, 0.06)",
    },
  };
});

const theme = computed(() => {
  if (settings.isDark) return darkTheme;
  return null;
});

// 同步 data-theme 到 <html>，驱动 CSS 变量切换
watch(
  () => settings.isDark,
  (isDark) => {
    if (typeof document !== "undefined") {
      document.documentElement.dataset.theme = isDark ? "dark" : "light";
    }
  },
  { immediate: true }
);

onMounted(async () => {
  // 初始化时也同步一次，确保首屏 data-theme 正确
  document.documentElement.dataset.theme = settings.isDark ? "dark" : "light";
  await settings.init();
  // init 后 config 加载完成，isDark 可能变化（如从默认 dark 切到 system/light），再同步一次
  document.documentElement.dataset.theme = settings.isDark ? "dark" : "light";
});
</script>

<template>
  <n-config-provider :theme="theme" :theme-overrides="themeOverrides">
    <n-global-style />
    <n-message-provider>
      <n-dialog-provider>
        <n-notification-provider>
          <router-view />
        </n-notification-provider>
      </n-dialog-provider>
    </n-message-provider>
  </n-config-provider>
</template>
