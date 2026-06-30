<script setup lang="ts">
import { computed, h } from "vue";
import { NIcon } from "naive-ui";
import {
  DesktopOutline as SystemIcon,
  SunnyOutline as LightIcon,
  MoonOutline as DarkIcon,
  ChevronDownOutline as ChevronIcon,
} from "@vicons/ionicons5";
import { useSettingsStore } from "@/stores/settings";

const settings = useSettingsStore();

type ThemeMode = "system" | "light" | "dark";

const currentMode = computed<ThemeMode>(() => settings.theme || "system");

const modeMeta: Record<
  ThemeMode,
  { label: string; icon: typeof SystemIcon }
> = {
  system: { label: "跟随系统", icon: SystemIcon },
  light: { label: "浅色", icon: LightIcon },
  dark: { label: "深色", icon: DarkIcon },
};

const currentIcon = computed(() => modeMeta[currentMode.value].icon);
const currentLabel = computed(() => modeMeta[currentMode.value].label);

const options = [
  {
    label: "跟随系统",
    key: "system",
    icon: () => h(NIcon, null, { default: () => h(SystemIcon) }),
  },
  {
    label: "浅色",
    key: "light",
    icon: () => h(NIcon, null, { default: () => h(LightIcon) }),
  },
  {
    label: "深色",
    key: "dark",
    icon: () => h(NIcon, null, { default: () => h(DarkIcon) }),
  },
];

function handleSelect(key: string) {
  const mode = key as ThemeMode;
  if (mode === currentMode.value) return;
  settings.setTheme(mode).catch((e) => {
    console.error("setTheme failed:", e);
  });
}
</script>

<template>
  <n-dropdown
    :options="options"
    trigger="click"
    size="small"
    @select="handleSelect"
  >
    <n-button quaternary circle :title="`主题：${currentLabel}`">
      <template #icon>
        <n-icon :component="currentIcon" size="20" />
      </template>
    </n-button>
  </n-dropdown>
</template>
