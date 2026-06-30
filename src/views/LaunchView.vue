<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useSettingsStore } from "@/stores/settings";
import { storeToRefs } from "pinia";
import WindowControls from "@/components/WindowControls.vue";
import ThemeToggle from "@/components/ThemeToggle.vue";
import {
  SettingsOutline as SettingsIcon,
  Play as PlayIcon,
  Stop as StopIcon,
  DocumentText as LogIcon,
  ChatbubbleEllipses as ChatIcon,
  CubeOutline as CubeIcon,
  OptionsOutline as OptionsIcon,
  RefreshOutline as RefreshIcon,
  AlertCircleOutline as AlertIcon,
  ServerOutline as ServerIcon,
  OpenOutline as OpenIcon,
  TrashOutline as TrashIcon,
  CloseOutline as DrawerCloseIcon,
} from "@vicons/ionicons5";

const router = useRouter();
const settings = useSettingsStore();
// 软件版本号（由 vite.config.ts 从 package.json 注入）
const appVersion = __APP_VERSION__;
// 选择状态提升到 store，路由切换（组件卸载/重建）时不会丢失
const { selectedPresetId, selectedModel } = storeToRefs(settings);

interface ModelInfo {
  path: string;
  file_name: string;
  folder: string;
  file_size_bytes: number;
  meta: any;
  parse_error: string | null;
}

interface ServerStatus {
  state: "stopped" | "starting" | "running" | "stopping" | "crashed";
  pid: number | null;
  port: number | null;
  host: string | null;
  model: string | null;
  started_at: number | null;
  message: string | null;
}

const models = ref<ModelInfo[]>([]);
const serverStatus = ref<ServerStatus>({
  state: "stopped",
  pid: null,
  port: null,
  host: null,
  model: null,
  started_at: null,
  message: null,
});
const logVisible = ref(false);
const logs = ref<{ ts: number; level: string; line: string }[]>([]);
const scanning = ref(false);

// 日志抽屉宽度响应式：大屏 600，小屏不超出窗口
const logDrawerWidth = ref(600);
function updateDrawerWidth() {
  logDrawerWidth.value = Math.min(600, Math.max(320, window.innerWidth - 40));
}
let resizeHandler: (() => void) | null = null;

const selectedModelInfo = computed(() =>
  models.value.find((m) => m.path === selectedModel.value)
);

const currentPreset = computed(
  () =>
    settings.presets.find((p) => p.id === selectedPresetId.value) ||
    settings.presets[0]
);

const isRunning = computed(
  () =>
    serverStatus.value.state === "running" ||
    serverStatus.value.state === "starting"
);

// 模型信息 stat 卡片数据
const modelStats = computed(() => {
  const m = selectedModelInfo.value;
  if (!m) return [];
  return [
    { label: "架构", value: m.meta?.architecture || "—" },
    { label: "量化", value: m.meta?.file_type_name || "—" },
    { label: "上下文", value: m.meta?.context_length ? Number(m.meta.context_length).toLocaleString() : "—" },
    { label: "层数", value: m.meta?.block_count || "—" },
    { label: "大小", value: formatBytes(m.file_size_bytes) },
    { label: "参数", value: m.meta?.parameter_count ? formatParams(m.meta.parameter_count) : "—" },
  ];
});

// 预设参数摘要 chips
const presetSummary = computed(() => {
  const p = currentPreset.value;
  if (!p) return [];
  const ngl =
    p.n_gpu_layers === "auto" ? "Auto" :
    p.n_gpu_layers === "all" ? "All" :
    String(p.n_gpu_layers);
  return [
    { label: "ctx", value: p.ctx_size || "默认" },
    { label: "ngl", value: ngl },
    { label: "port", value: String(p.port) },
    { label: "threads", value: p.threads < 0 ? "自动" : String(p.threads) },
    { label: "fa", value: p.flash_attn },
  ];
});

// 状态映射：颜色 + 文案 + 是否呼吸
const statusInfo = computed(() => {
  const s = serverStatus.value.state;
  switch (s) {
    case "running":
      return { color: "#10b981", text: "运行中", pulse: true };
    case "starting":
      return { color: "#f59e0b", text: "启动中", pulse: true };
    case "stopping":
      return { color: "#f59e0b", text: "停止中", pulse: true };
    case "crashed":
      return { color: "#ef4444", text: "已崩溃", pulse: false };
    default:
      return { color: "#71717a", text: "未运行", pulse: false };
  }
});

const canStart = computed(
  () => !!selectedModel.value && !!settings.config?.server_path && !isRunning.value
);

function formatParams(n: number): string {
  if (n >= 1e9) return (n / 1e9).toFixed(1) + "B";
  if (n >= 1e6) return (n / 1e6).toFixed(1) + "M";
  return String(n);
}

let unlistenLog: UnlistenFn | null = null;
let unlistenStatus: UnlistenFn | null = null;

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return (bytes / Math.pow(k, i)).toFixed(2) + " " + sizes[i];
}

async function scanModels() {
  if (!settings.config?.model_folders.length) return;
  scanning.value = true;
  try {
    models.value = await invoke<ModelInfo[]>("scan_models", {
      folders: settings.config.model_folders,
    });
    // 校验当前选中（来自 store 的 last_used）是否仍存在；存在则保留，否则 fallback
    const current = selectedModel.value;
    if (current && models.value.some((m) => m.path === current)) {
      // 保持已选
    } else if (models.value.length > 0) {
      selectedModel.value = models.value[0].path;
    } else {
      selectedModel.value = null;
    }
  } catch (e) {
    console.error("Scan failed:", e);
  } finally {
    scanning.value = false;
  }
}

async function startServer() {
  if (!selectedModel.value || !currentPreset.value || !settings.config?.server_path) return;
  try {
    // 先持久化"上次使用"的预设和模型，下次启动时自动恢复
    try {
      await settings.persistLastUsed(currentPreset.value.id, selectedModel.value);
    } catch (err) {
      console.error("Persist last used failed:", err);
    }
    await invoke("start_server", {
      modelPath: selectedModel.value,
      preset: currentPreset.value,
      serverPath: settings.config.server_path,
    });
  } catch (e) {
    console.error("Start failed:", e);
    alert(String(e));
  }
}

async function stopServer() {
  try {
    await invoke("stop_server");
  } catch (e) {
    console.error("Stop failed:", e);
  }
}

async function openChat() {
  if (serverStatus.value.port && serverStatus.value.host) {
    await invoke("open_chat_window", {
      url: `http://${serverStatus.value.host}:${serverStatus.value.port}`,
    });
  }
}

async function refreshStatus() {
  serverStatus.value = await invoke<ServerStatus>("server_status");
}

onMounted(async () => {
  // 监听日志事件
  unlistenLog = await listen<{ ts: number; level: string; line: string }>(
    "server://log",
    (event) => {
      logs.value.push(event.payload);
      if (logs.value.length > 5000) logs.value.shift();
    }
  );
  // 监听状态事件
  unlistenStatus = await listen<ServerStatus>("server://status", (event) => {
    serverStatus.value = event.payload;
  });
  await refreshStatus();
  // 确保 config 已加载（子组件 mounted 可能早于父组件的 init），选择状态由 store.init 恢复
  if (!settings.config) {
    await settings.init();
  }
  // 扫描模型（内部会校验 selectedModel 是否存在）
  await scanModels();

  // 日志抽屉宽度响应式
  updateDrawerWidth();
  resizeHandler = () => updateDrawerWidth();
  window.addEventListener("resize", resizeHandler);
});

onUnmounted(() => {
  unlistenLog?.();
  unlistenStatus?.();
  if (resizeHandler) {
    window.removeEventListener("resize", resizeHandler);
    resizeHandler = null;
  }
});
</script>

<template>
  <div class="launch-view">
    <!-- 顶部栏 -->
    <header class="app-header" data-tauri-drag-region>
      <div class="header-left" data-tauri-drag-region="false">
        <div class="brand">
          <div class="brand-mark">
            <n-icon :component="ServerIcon" size="18" />
          </div>
          <span class="app-title">llama.cpp RGUI</span>
          <span class="app-version">v{{ appVersion }}</span>
        </div>
      </div>
      <div class="header-right" data-tauri-drag-region="false">
        <n-tag
          v-if="settings.config?.server_version"
          size="small"
          round
          :bordered="false"
          class="ver-tag"
        >
          llama-server v{{ settings.config.server_version }}
        </n-tag>
        <n-tag v-else size="small" type="warning" round :bordered="false">
          未配置 llama-server
        </n-tag>
        <n-button quaternary circle @click="router.push('/settings')">
          <template #icon>
            <n-icon :component="SettingsIcon" size="20" />
          </template>
        </n-button>
        <ThemeToggle />
        <WindowControls />
      </div>
    </header>

    <!-- 主内容区 -->
    <main class="main-content">
      <div class="container">
        <!-- 未配置 server 引导 -->
        <div v-if="!settings.config?.server_path" class="guide-banner">
          <n-icon :component="AlertIcon" size="18" />
          <span>未配置 llama-server 路径，请先前往设置选择可执行文件</span>
          <n-button size="tiny" tertiary type="primary" @click="router.push('/settings')">
            去设置
          </n-button>
        </div>

        <!-- 模型面板 -->
        <section class="panel">
          <div class="panel-head">
            <div class="panel-title">
              <n-icon :component="CubeIcon" size="18" />
              <span>模型</span>
            </div>
            <n-button text size="small" :loading="scanning" @click="scanModels">
              <template #icon>
                <n-icon :component="RefreshIcon" />
              </template>
              刷新
            </n-button>
          </div>

          <n-select
            v-model:value="selectedModel"
            :options="models.map((m) => ({ label: m.file_name, value: m.path }))"
            placeholder="选择模型"
            filterable
            :loading="scanning"
            size="large"
            class="model-select"
          />

          <!-- 模型信息 stat 网格 -->
          <div v-if="selectedModelInfo" class="stat-grid">
            <div v-for="s in modelStats" :key="s.label" class="stat">
              <div class="stat-label">{{ s.label }}</div>
              <div class="stat-value">{{ s.value }}</div>
            </div>
          </div>

          <div v-else-if="models.length === 0 && !scanning" class="empty-hint">
            暂无模型，请先在设置中添加模型文件夹
          </div>

          <n-alert
            v-if="selectedModelInfo?.parse_error"
            type="warning"
            :show-icon="true"
            class="parse-alert"
          >
            元数据解析失败：{{ selectedModelInfo.parse_error }}
          </n-alert>
        </section>

        <!-- 预设面板 -->
        <section class="panel">
          <div class="panel-head">
            <div class="panel-title">
              <n-icon :component="OptionsIcon" size="18" />
              <span>启动预设</span>
            </div>
          </div>
          <n-select
            v-model:value="selectedPresetId"
            :options="settings.presets.map((p) => ({ label: p.name, value: p.id }))"
            size="large"
            class="preset-select"
          />
          <div v-if="currentPreset" class="preset-summary">
            <span v-for="s in presetSummary" :key="s.label" class="chip">
              <span class="chip-label">{{ s.label }}</span>
              <span class="chip-value">{{ s.value }}</span>
            </span>
          </div>
        </section>

        <!-- 启动控制区 -->
        <section class="control-panel" :class="{ running: isRunning }">
          <button
            class="launch-btn"
            :class="{ disabled: !canStart && !isRunning }"
            :disabled="!canStart && !isRunning"
            @click="isRunning ? stopServer() : startServer()"
          >
            <div class="launch-btn-bg"></div>
            <div class="launch-btn-content">
              <n-icon :component="isRunning ? StopIcon : PlayIcon" size="22" />
              <span>{{ isRunning ? "停止服务器" : "启动服务器" }}</span>
            </div>
          </button>

          <!-- 状态行 -->
          <div class="status-row">
            <div class="status-led" :class="{ pulse: statusInfo.pulse }" :style="{ background: statusInfo.color, boxShadow: `0 0 8px ${statusInfo.color}` }"></div>
            <span class="status-text">{{ statusInfo.text }}</span>
            <template v-if="serverStatus.pid">
              <span class="status-sep">·</span>
              <span class="status-text">PID {{ serverStatus.pid }}</span>
            </template>
            <template v-if="serverStatus.port">
              <span class="status-sep">·</span>
              <span class="status-text mono">{{ serverStatus.host }}:{{ serverStatus.port }}</span>
            </template>

            <div class="status-actions">
              <n-button
                v-if="serverStatus.state === 'running'"
                quaternary
                size="small"
                @click="openChat"
              >
                <template #icon>
                  <n-icon :component="OpenIcon" />
                </template>
                打开聊天
              </n-button>
              <n-button quaternary size="small" @click="logVisible = true">
                <template #icon>
                  <n-icon :component="LogIcon" />
                </template>
                日志
                <n-badge
                  v-if="logs.length > 0"
                  :value="logs.length"
                  :max="99"
                  type="info"
                  style="margin-left: 4px"
                />
              </n-button>
            </div>
          </div>
        </section>
      </div>
    </main>

    <!-- 日志抽屉 -->
    <n-drawer
      v-model:show="logVisible"
      :width="logDrawerWidth"
      placement="right"
      :auto-focus="false"
      class="log-drawer"
    >
      <n-drawer-content
        :native-scrollbar="false"
        :closable="false"
        body-content-style="padding: 0;"
        header-style="padding: 12px 16px;"
        body-style="padding: 0;"
      >
        <template #header>
          <div class="drawer-header" data-tauri-drag-region>
            <div class="drawer-title" data-tauri-drag-region>
              <n-icon :component="LogIcon" size="16" />
              <span>运行日志</span>
              <n-tag
                v-if="logs.length > 0"
                size="tiny"
                round
                :bordered="false"
                class="log-count-tag"
                data-tauri-drag-region="false"
              >
                {{ logs.length }}
              </n-tag>
            </div>
            <div class="drawer-actions" data-tauri-drag-region="false">
              <n-button
                text
                size="small"
                :disabled="logs.length === 0"
                @click="logs = []"
              >
                <template #icon>
                  <n-icon :component="TrashIcon" size="16" />
                </template>
                清空
              </n-button>
              <n-button
                quaternary
                circle
                size="small"
                @click="logVisible = false"
              >
                <template #icon>
                  <n-icon :component="DrawerCloseIcon" size="18" />
                </template>
              </n-button>
            </div>
          </div>
        </template>
        <div class="log-body">
          <n-log
            :log="logs.map((l) => l.line).join('\n')"
            :rows="30"
            trim
            :font-size="13"
            class="log-viewer"
          />
          <div v-if="logs.length === 0" class="log-empty">
            <n-icon :component="LogIcon" size="32" />
            <span>暂无日志</span>
          </div>
        </div>
      </n-drawer-content>
    </n-drawer>
  </div>
</template>

<style scoped>
/* === 根容器：固定 header + 可滚动 main，最朴素可靠的方案 === */
.launch-view {
  height: 100vh;
  display: flex;
  flex-direction: column;
  position: relative;
  overflow: hidden;
}

/* 背景光晕（absolute，不影响布局流） */
.launch-view::before {
  content: "";
  position: absolute;
  top: -200px;
  left: 50%;
  transform: translateX(-50%);
  width: 800px;
  height: 500px;
  background: radial-gradient(ellipse at center, rgba(99, 102, 241, 0.12), transparent 70%);
  pointer-events: none;
  z-index: 0;
}

/* === Header：固定不收缩 === */
.app-header {
  position: relative;
  z-index: 2;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 20px;
  border-bottom: 1px solid var(--border-base);
  backdrop-filter: blur(8px);
  transition: border-color 0.25s ease;
}

.header-left .brand {
  display: flex;
  align-items: center;
  gap: 10px;
}

.brand-mark {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #6366f1, #8b5cf6);
  color: #fff;
  box-shadow: 0 2px 8px rgba(99, 102, 241, 0.35);
}

.app-title {
  font-size: 15px;
  font-weight: 600;
  letter-spacing: 0.2px;
  color: var(--text-base);
}

/* 软件版本号（紧贴标题右侧，弱化显示） */
.app-version {
  font-size: 11px;
  font-weight: 500;
  color: var(--text-muted);
  padding: 1px 7px;
  border-radius: 999px;
  background: var(--surface-bg);
  border: 1px solid var(--surface-border);
  letter-spacing: 0.3px;
  font-variant-numeric: tabular-nums;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.ver-tag {
  background: var(--accent-bg);
  color: var(--accent-text);
  font-weight: 500;
}

/* === 主内容：可滚动区域（关键修复） ===
   不再用 flex 垂直居中（margin:auto 0 是滚动 bug 根源），
   改为 block 布局 + margin:0 auto 水平居中，内容从顶部排列，超出即滚动。 */
.main-content {
  position: relative;
  z-index: 1;
  flex: 1;
  min-height: 0; /* flex 子项允许收缩，overflow 才能生效 */
  overflow-y: auto;
  overflow-x: hidden;
  padding: 28px 20px 40px;
}

.container {
  width: 100%;
  max-width: 680px;
  margin: 0 auto; /* 仅水平居中，不碰垂直 */
  display: flex;
  flex-direction: column;
  gap: 16px;
}

/* 引导横幅 */
.guide-banner {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  border-radius: 12px;
  background: rgba(245, 158, 11, 0.1);
  border: 1px solid rgba(245, 158, 11, 0.25);
  color: #fbbf24;
  font-size: 13px;
}
.guide-banner > span {
  flex: 1;
}

/* === Panel === */
.panel {
  padding: 18px 20px;
  border-radius: 14px;
  background: var(--panel-bg);
  border: 1px solid var(--panel-border);
  transition: background-color 0.25s ease, border-color 0.25s ease;
}

.panel-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 14px;
}

.panel-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-base);
}

.model-select,
.preset-select {
  --n-border-radius: 10px;
}

/* === 模型 stat 网格 === */
.stat-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
  margin-top: 14px;
}

.stat {
  padding: 10px 12px;
  border-radius: 10px;
  background: var(--surface-bg);
  border: 1px solid var(--surface-border);
  transition: background-color 0.25s ease, border-color 0.25s ease;
}

.stat-label {
  font-size: 11px;
  color: var(--text-muted);
  margin-bottom: 4px;
  letter-spacing: 0.3px;
}

.stat-value {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-base);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.empty-hint {
  margin-top: 14px;
  padding: 16px;
  text-align: center;
  font-size: 13px;
  color: var(--text-faint);
  border-radius: 10px;
  border: 1px dashed var(--border-strong);
}

.parse-alert {
  margin-top: 12px;
  border-radius: 10px;
}

/* === 预设摘要 chips === */
.preset-summary {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 14px;
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  border-radius: 999px;
  background: var(--accent-bg);
  border: 1px solid var(--accent-border);
  font-size: 12px;
  transition: background-color 0.25s ease, border-color 0.25s ease;
}

.chip-label {
  color: var(--text-muted);
}

.chip-value {
  font-weight: 600;
  color: var(--accent-text);
}

/* === 启动控制区 === */
.control-panel {
  padding: 20px;
  border-radius: 14px;
  background: var(--panel-bg);
  border: 1px solid var(--panel-border);
  transition: background-color 0.25s ease, border-color 0.25s ease;
}
.control-panel.running {
  border-color: rgba(239, 68, 68, 0.25);
  background: rgba(239, 68, 68, 0.04);
}

.launch-btn {
  position: relative;
  width: 100%;
  height: 56px;
  border: none;
  border-radius: 12px;
  cursor: pointer;
  overflow: hidden;
  padding: 0;
}
.launch-btn.disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

.launch-btn-bg {
  position: absolute;
  inset: 0;
  background: linear-gradient(135deg, #6366f1, #8b5cf6);
}
.control-panel.running .launch-btn-bg {
  background: linear-gradient(135deg, #ef4444, #f97316);
}

.launch-btn-content {
  position: relative;
  z-index: 1;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  color: #fff;
  font-size: 16px;
  font-weight: 600;
  letter-spacing: 0.5px;
}

/* === 状态行 === */
.status-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 14px;
  flex-wrap: wrap;
}

.status-led {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.status-led.pulse {
  animation: pulse 1.6s ease-in-out infinite;
}
@keyframes pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.5; transform: scale(0.8); }
}

.status-text {
  font-size: 13px;
  color: var(--text-muted);
}
.status-text.mono {
  font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
}
.status-sep {
  color: var(--text-faint);
}

.status-actions {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 4px;
}

/* === 日志抽屉：与应用整体融合 ===
   - top 偏移留出应用 header 高度，避免遮盖窗口控件
   - 左上圆角与窗口边缘视觉融合
   - 半透明背景 + 毛玻璃，与 LaunchView 面板风格一致 */
:deep(.log-drawer.n-drawer) {
  top: 49px;
  border-top-left-radius: 14px;
  border-bottom-left-radius: 14px;
  overflow: hidden;
  background: var(--drawer-bg);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  box-shadow: -8px 0 32px rgba(0, 0, 0, 0.4);
  transition: background-color 0.25s ease;
}

/* drawer mask 轻微透明，让底层内容隐约可见 */
:deep(.n-drawer-mask) {
  background: var(--mask-bg);
  backdrop-filter: blur(2px);
  -webkit-backdrop-filter: blur(2px);
}

.drawer-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  gap: 12px;
}

.drawer-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-base);
  flex: 1;
  min-width: 0;
}

.log-count-tag {
  background: var(--accent-bg);
  color: var(--accent-text);
  font-weight: 600;
}

.drawer-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}

.log-body {
  position: relative;
  height: 100%;
}

/* n-log 自身高度撑满，字号/行距微调 */
:deep(.log-viewer.n-log) {
  height: 100%;
  font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
  --n-font-size: 13px;
}

.log-empty {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: var(--text-faint);
  font-size: 13px;
  pointer-events: none;
}
</style>
