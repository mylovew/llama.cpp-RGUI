import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface Preset {
  id: string;
  name: string;
  ctx_size: number;
  n_gpu_layers: "count" | "auto" | "all" | number;
  threads: number;
  host: string;
  port: number;
  alias: string | null;
  flash_attn: "on" | "off" | "auto";
  batch_size: number;
  ubatch_size: number;
  parallel: number;
  cache_type_k: string;
  cache_type_v: string;
  cont_batching: boolean;
  api_key: string | null;
  split_mode: "none" | "layer" | "row";
  main_gpu: number | null;
  jinja: boolean;
  chat_template: string | null;
  custom_args: string[];
  extra_env: Record<string, string>;
}

export interface AppConfig {
  server_path: string | null;
  server_version: string | null;
  model_folders: string[];
  default_model_path: string | null;
  default_preset_id: string | null;
  presets: Preset[];
  last_used_preset_id: string | null;
  last_used_model_path: string | null;
  theme: "light" | "dark" | "system";
  window: { width: number; height: number; x: number | null; y: number | null; maximized: boolean };
}

export const useSettingsStore = defineStore("settings", () => {
  const config = ref<AppConfig | null>(null);
  const loading = ref(false);

  // 提升到 store 级别的选择状态，避免路由切换（组件卸载/重建）导致丢失
  const selectedPresetId = ref<string | null>(null);
  const selectedModel = ref<string | null>(null);

  // 系统主题偏好（响应系统主题变化）
  const systemPrefersDark = ref(
    typeof window !== "undefined" &&
      window.matchMedia &&
      window.matchMedia("(prefers-color-scheme: dark)").matches
  );

  // 监听系统主题变化（仅在 theme === "system" 时生效，但始终维护此 ref）
  let mediaQuery: MediaQueryList | null = null;
  let mediaHandler: ((e: MediaQueryListEvent) => void) | null = null;
  if (typeof window !== "undefined" && window.matchMedia) {
    mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    mediaHandler = (e: MediaQueryListEvent) => {
      systemPrefersDark.value = e.matches;
    };
    // addEventListener 在现代浏览器可用；Safari < 14 需要 addListener
    if (mediaQuery.addEventListener) {
      mediaQuery.addEventListener("change", mediaHandler);
    } else if ((mediaQuery as any).addListener) {
      (mediaQuery as any).addListener(mediaHandler);
    }
  }

  // init 幂等守卫：App.vue 与 LaunchView 都可能调用，确保只执行一次 load_config
  let initPromise: Promise<void> | null = null;

  const isDark = computed(() => {
    if (!config.value) return systemPrefersDark.value;
    if (config.value.theme === "dark") return true;
    if (config.value.theme === "light") return false;
    // system
    return systemPrefersDark.value;
  });

  const theme = computed(() => config.value?.theme ?? "system");

  const presets = computed(() => config.value?.presets ?? []);

  async function init() {
    if (initPromise) return initPromise;
    initPromise = (async () => {
      loading.value = true;
      try {
        config.value = await invoke<AppConfig>("load_config");
        // 加载完成后初始化上次使用的预设和模型
        const cfg = config.value;
        selectedPresetId.value =
          cfg.last_used_preset_id ||
          cfg.default_preset_id ||
          cfg.presets[0]?.id ||
          null;
        selectedModel.value =
          cfg.last_used_model_path || cfg.default_model_path;
      } catch (e) {
        console.error("Failed to load config:", e);
      } finally {
        loading.value = false;
      }
    })();
    return initPromise;
  }

  async function save(newConfig: AppConfig) {
    await invoke("save_config", { newConfig });
    config.value = newConfig;
  }

  // 持久化“上次使用”的预设和模型（在启动服务器时调用）
  async function persistLastUsed(presetId: string, modelPath: string) {
    if (!config.value) return;
    const newConfig: AppConfig = {
      ...config.value,
      last_used_preset_id: presetId,
      last_used_model_path: modelPath,
    };
    await save(newConfig);
  }

  // 切换主题并持久化
  async function setTheme(newTheme: "light" | "dark" | "system") {
    if (!config.value) return;
    const newConfig: AppConfig = { ...config.value, theme: newTheme };
    await save(newConfig);
  }

  return {
    config,
    loading,
    isDark,
    theme,
    systemPrefersDark,
    presets,
    selectedPresetId,
    selectedModel,
    init,
    save,
    persistLastUsed,
    setTheme,
  };
});
