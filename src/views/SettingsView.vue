<script setup lang="ts">
import { ref, computed, watch, onMounted } from "vue";
import { useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { useDialog } from "naive-ui";
import { useSettingsStore, type Preset, type AppConfig } from "@/stores/settings";
import WindowControls from "@/components/WindowControls.vue";
import {
  ArrowBack as BackIcon,
  Add as AddIcon,
  Trash as TrashIcon,
  Copy as CopyIcon,
  FolderOpen as FolderIcon,
  RefreshOutline as RefreshIcon,
  ChevronDownOutline as ChevronIcon,
  CubeOutline as CubeIcon,
  ServerOutline as ServerIcon,
  OptionsOutline as OptionsIcon,
  CloudDownloadOutline as UpdateIcon,
  CheckmarkCircleOutline as OkIcon,
  AlertCircleOutline as AlertIcon,
  OpenOutline as OpenIcon,
} from "@vicons/ionicons5";

const router = useRouter();
const settings = useSettingsStore();
const dialog = useDialog();

const activeTab = ref("server");

// === llama-server 路径 ===
const serverPath = ref<string | null>(null);
const serverVersion = ref<string | null>(null);
const detecting = ref(false);

// === 版本更新检查 ===
interface VersionCheckResult {
  current: string | null;
  current_number: number | null;
  latest_tag: string;
  latest_number: number | null;
  has_update: boolean;
  release_url: string;
  published_at: string | null;
  error: string | null;
}
const versionCheck = ref<VersionCheckResult | null>(null);
const checkingUpdate = ref(false);

async function pickServer() {
  const path = await invoke<string | null>("pick_file", { filter: null });
  if (path) {
    serverPath.value = path;
    versionCheck.value = null;
    await detectVersion();
    await persistCurrent();
  }
}

async function detectVersion() {
  if (!serverPath.value) return;
  detecting.value = true;
  try {
    serverVersion.value = await invoke<string>("detect_server_version", {
      serverPath: serverPath.value,
    });
    await persistCurrent();
  } catch (e) {
    serverVersion.value = null;
    console.error(e);
  } finally {
    detecting.value = false;
  }
}

async function checkUpdate() {
  if (!serverVersion.value) {
    await detectVersion();
  }
  checkingUpdate.value = true;
  try {
    versionCheck.value = await invoke<VersionCheckResult>("check_latest_version", {
      currentVersion: serverVersion.value,
    });
  } catch (e) {
    versionCheck.value = {
      current: serverVersion.value,
      current_number: null,
      latest_tag: "",
      latest_number: null,
      has_update: false,
      release_url: "",
      published_at: null,
      error: String(e),
    };
  } finally {
    checkingUpdate.value = false;
  }
}

async function openUrl(url: string) {
  try {
    await invoke("open_chat_window", { url });
  } catch (e) {
    console.error(e);
  }
}

// === 模型文件夹 ===
const modelFolders = ref<string[]>([]);

interface ModelInfo {
  path: string;
  file_name: string;
  folder: string;
  file_size_bytes: number;
  meta: any;
  parse_error: string | null;
}

const allModels = ref<ModelInfo[]>([]);
const scanningModels = ref(false);
const expandedFolders = ref<Set<string>>(new Set());

// 按文件夹分组
const foldersWithModels = computed(() => {
  return modelFolders.value.map((folder) => {
    const models = allModels.value.filter((m) => m.folder === folder);
    return { folder, models };
  });
});

async function addFolder() {
  const path = await invoke<string | null>("pick_folder");
  if (path && !modelFolders.value.includes(path)) {
    modelFolders.value.push(path);
    expandedFolders.value.add(path);
    await persistCurrent();
    await scanModels();
  }
}

function removeFolder(index: number) {
  const folder = modelFolders.value[index];
  dialog.warning({
    title: "移除文件夹",
    content: `确定要从列表中移除「${folderBasename(folder)}」吗？此操作不会删除磁盘上的文件，仅从扫描列表中移除。`,
    positiveText: "移除",
    negativeText: "取消",
    onPositiveClick: async () => {
      modelFolders.value.splice(index, 1);
      expandedFolders.value.delete(folder);
      allModels.value = allModels.value.filter((m) => m.folder !== folder);
      await persistCurrent();
    },
  });
}

function toggleFolder(folder: string) {
  if (expandedFolders.value.has(folder)) {
    expandedFolders.value.delete(folder);
  } else {
    expandedFolders.value.add(folder);
  }
}

async function scanModels() {
  if (!modelFolders.value.length) {
    allModels.value = [];
    return;
  }
  scanningModels.value = true;
  try {
    allModels.value = await invoke<ModelInfo[]>("scan_models", {
      folders: modelFolders.value,
    });
  } catch (e) {
    console.error("Scan failed:", e);
    allModels.value = [];
  } finally {
    scanningModels.value = false;
  }
}

function folderBasename(folder: string): string {
  return folder.split(/[\\/]/).pop() || folder;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return (bytes / Math.pow(k, i)).toFixed(2) + " " + sizes[i];
}

// === 预设编辑 ===
const editingPreset = ref<Preset | null>(null);
const presetList = computed(() => settings.config?.presets ?? []);

function newPreset() {
  const p: Preset = {
    id: crypto.randomUUID(),
    name: "新预设",
    ctx_size: 0,
    n_gpu_layers: "auto",
    threads: -1,
    host: "127.0.0.1",
    port: 8080,
    alias: null,
    flash_attn: "auto",
    batch_size: 2048,
    ubatch_size: 512,
    parallel: 1,
    cache_type_k: "F16",
    cache_type_v: "F16",
    cont_batching: true,
    api_key: null,
    split_mode: "layer",
    main_gpu: null,
    jinja: true,
    chat_template: null,
    custom_args: [],
    extra_env: {},
  };
  editingPreset.value = p;
}

function editPreset(p: Preset) {
  editingPreset.value = JSON.parse(JSON.stringify(p));
}

function duplicatePreset(p: Preset) {
  const dup = JSON.parse(JSON.stringify(p));
  dup.id = crypto.randomUUID();
  dup.name = p.name + " 副本";
  editingPreset.value = dup;
}

function deletePreset(id: string) {
  if (!settings.config || settings.config.presets.length <= 1) return;
  const p = settings.config.presets.find((x) => x.id === id);
  dialog.warning({
    title: "删除预设",
    content: `确定要删除预设「${p?.name ?? ""}」吗？此操作不可撤销。`,
    positiveText: "删除",
    negativeText: "取消",
    onPositiveClick: async () => {
      if (!settings.config) return;
      settings.config.presets = settings.config.presets.filter((x) => x.id !== id);
      if (editingPreset.value?.id === id) editingPreset.value = null;
      if (settings.selectedPresetId === id) {
        settings.selectedPresetId = settings.config.presets[0]?.id ?? null;
      }
      await persistCurrent();
    },
  });
}

async function savePreset() {
  if (!editingPreset.value || !settings.config) return;
  const idx = settings.config.presets.findIndex((p) => p.id === editingPreset.value!.id);
  if (idx >= 0) {
    settings.config.presets[idx] = editingPreset.value;
  } else {
    settings.config.presets.push(editingPreset.value);
  }
  editingPreset.value = null;
  await persistCurrent();
}

// === 保存 ===
watch(
  () => settings.config,
  (cfg) => {
    if (cfg) {
      serverPath.value = cfg.server_path;
      serverVersion.value = cfg.server_version;
      modelFolders.value = [...cfg.model_folders];
    }
  },
  { immediate: true }
);

// 即时持久化：把当前编辑的本地 ref 状态写回 config 并保存到磁盘
async function persistCurrent() {
  if (!settings.config) return;
  const cfg: AppConfig = {
    ...settings.config,
    server_path: serverPath.value,
    server_version: serverVersion.value,
    model_folders: modelFolders.value,
  };
  await settings.save(cfg);
}

// 进入文件夹 tab 时自动扫描一次
watch(activeTab, (tab) => {
  if (tab === "folders" && allModels.value.length === 0 && modelFolders.value.length > 0) {
    scanModels();
  }
});

onMounted(() => {
  if (modelFolders.value.length > 0) {
    scanModels();
  }
});
</script>

<template>
  <div class="settings-view">
    <!-- 顶部栏 -->
    <header class="settings-header" data-tauri-drag-region>
      <div class="header-left" data-tauri-drag-region="false">
        <n-button quaternary circle @click="router.push('/')">
          <template #icon>
            <n-icon :component="BackIcon" size="20" />
          </template>
        </n-button>
        <span class="header-title">设置</span>
        <span class="header-hint">修改即时生效</span>
      </div>
      <div class="header-right" data-tauri-drag-region="false">
        <WindowControls />
      </div>
    </header>

    <!-- 顶部浮动胶囊导航 -->
    <nav class="tab-nav">
      <div class="tab-nav-inner">
        <button
          v-for="t in [
            { key: 'server', label: 'llama-server', icon: ServerIcon },
            { key: 'folders', label: '模型文件夹', icon: CubeIcon },
            { key: 'presets', label: '启动预设', icon: OptionsIcon },
          ]"
          :key="t.key"
          class="tab-btn"
          :class="{ active: activeTab === t.key }"
          @click="activeTab = t.key"
        >
          <n-icon :component="t.icon" size="16" />
          <span>{{ t.label }}</span>
        </button>
      </div>
    </nav>

    <main class="settings-main">
      <!-- ============ Tab: llama-server ============ -->
      <div v-show="activeTab === 'server'" class="tab-pane">
        <div class="container">
          <!-- 程序路径 panel -->
          <section class="panel">
            <div class="panel-head">
              <div class="panel-title">
                <n-icon :component="ServerIcon" size="18" />
                <span>llama-server 程序</span>
              </div>
            </div>
            <n-input-group>
              <n-input
                :value="serverPath || ''"
                placeholder="选择 llama-server 可执行文件路径"
                readonly
                size="large"
              />
              <n-button size="large" type="primary" @click="pickServer">
                <template #icon>
                  <n-icon :component="FolderIcon" />
                </template>
                选择
              </n-button>
            </n-input-group>
          </section>

          <!-- 版本 & 更新检测 panel -->
          <section class="panel">
            <div class="panel-head">
              <div class="panel-title">
                <n-icon :component="UpdateIcon" size="18" />
                <span>版本与更新</span>
              </div>
              <n-button
                size="small"
                tertiary
                :loading="detecting"
                @click="detectVersion"
              >
                检测本地版本
              </n-button>
            </div>

            <!-- 本地版本 -->
            <div class="version-row">
              <span class="version-label">本地版本</span>
              <n-tag
                v-if="serverVersion"
                size="small"
                round
                :bordered="false"
                class="ver-tag"
              >
                {{ serverVersion }}
              </n-tag>
              <span v-else class="version-empty">未检测</span>
            </div>

            <!-- 更新检查 -->
            <div class="update-section">
              <n-button
                size="small"
                type="primary"
                ghost
                :loading="checkingUpdate"
                :disabled="!serverPath"
                @click="checkUpdate"
              >
                <template #icon>
                  <n-icon :component="UpdateIcon" />
                </template>
                检查 GitHub 最新版本
              </n-button>

              <!-- 检查结果 -->
              <div v-if="versionCheck" class="update-result">
                <template v-if="versionCheck.error">
                  <div class="update-card error">
                    <n-icon :component="AlertIcon" size="18" />
                    <div class="update-card-body">
                      <div class="update-card-title">检查失败</div>
                      <div class="update-card-desc">{{ versionCheck.error }}</div>
                    </div>
                  </div>
                </template>
                <template v-else-if="versionCheck.has_update">
                  <div class="update-card warn">
                    <n-icon :component="AlertIcon" size="20" />
                    <div class="update-card-body">
                      <div class="update-card-title">
                        发现新版本
                        <n-tag size="tiny" round :bordered="false" type="warning">
                          {{ versionCheck.latest_tag }}
                        </n-tag>
                      </div>
                      <div class="update-card-desc">
                        当前 {{ versionCheck.current || "—" }} → 最新 {{ versionCheck.latest_tag }}
                        <span v-if="versionCheck.published_at" class="pub-time">
                          · 发布于 {{ versionCheck.published_at.slice(0, 10) }}
                        </span>
                      </div>
                    </div>
                    <n-button
                      size="small"
                      type="warning"
                      @click="openUrl(versionCheck.release_url)"
                    >
                      <template #icon>
                        <n-icon :component="OpenIcon" />
                      </template>
                      前往下载
                    </n-button>
                  </div>
                </template>
                <template v-else-if="versionCheck.latest_tag">
                  <div class="update-card ok">
                    <n-icon :component="OkIcon" size="20" />
                    <div class="update-card-body">
                      <div class="update-card-title">
                        已是最新版本
                        <n-tag size="tiny" round :bordered="false" type="success">
                          {{ versionCheck.latest_tag }}
                        </n-tag>
                      </div>
                      <div class="update-card-desc">
                        本地 {{ versionCheck.current || "—" }}
                        <span v-if="versionCheck.published_at" class="pub-time">
                          · 发布于 {{ versionCheck.published_at.slice(0, 10) }}
                        </span>
                      </div>
                    </div>
                    <n-button
                      size="small"
                      quaternary
                      @click="openUrl(versionCheck.release_url)"
                    >
                      <template #icon>
                        <n-icon :component="OpenIcon" />
                      </template>
                      查看 Releases
                    </n-button>
                  </div>
                </template>
              </div>
              <p v-else class="update-hint">
                点击上方按钮，对比 GitHub 上 llama.cpp 的最新 release 版本
              </p>
            </div>
          </section>
        </div>
      </div>

      <!-- ============ Tab: 模型文件夹 ============ -->
      <div v-show="activeTab === 'folders'" class="tab-pane">
        <div class="container">
          <section class="panel">
            <div class="panel-head">
              <div class="panel-title">
                <n-icon :component="CubeIcon" size="18" />
                <span>模型文件夹</span>
                <n-tag
                  v-if="allModels.length > 0"
                  size="tiny"
                  round
                  :bordered="false"
                  class="count-tag"
                >
                  {{ allModels.length }} 个模型
                </n-tag>
              </div>
              <div class="panel-actions">
                <n-button
                  size="small"
                  tertiary
                  :loading="scanningModels"
                  :disabled="modelFolders.length === 0"
                  @click="scanModels"
                >
                  <template #icon>
                    <n-icon :component="RefreshIcon" />
                  </template>
                  刷新扫描
                </n-button>
                <n-button size="small" type="primary" @click="addFolder">
                  <template #icon>
                    <n-icon :component="AddIcon" />
                  </template>
                  添加文件夹
                </n-button>
              </div>
            </div>

            <!-- 文件夹列表 -->
            <div v-if="modelFolders.length === 0" class="empty-folders">
              <n-icon :component="FolderIcon" size="32" />
              <p>暂无文件夹</p>
              <span>点击右上角「添加文件夹」开始管理模型</span>
            </div>

            <div v-else class="folder-list">
              <div
                v-for="(item, i) in foldersWithModels"
                :key="item.folder"
                class="folder-card"
              >
                <!-- 文件夹头 -->
                <div class="folder-head" @click="toggleFolder(item.folder)">
                  <n-icon
                    :component="ChevronIcon"
                    size="16"
                    class="chevron"
                    :class="{ expanded: expandedFolders.has(item.folder) }"
                  />
                  <n-icon :component="FolderIcon" size="18" class="folder-ico" />
                  <div class="folder-info">
                    <div class="folder-name">{{ folderBasename(item.folder) }}</div>
                    <div class="folder-path">{{ item.folder }}</div>
                  </div>
                  <n-tag size="tiny" round :bordered="false" class="count-tag">
                    {{ item.models.length }}
                  </n-tag>
                  <div class="folder-actions" @click.stop>
                    <n-button
                      quaternary
                      circle
                      size="tiny"
                      :loading="scanningModels"
                      @click="scanModels"
                    >
                      <template #icon>
                        <n-icon :component="RefreshIcon" size="14" />
                      </template>
                    </n-button>
                    <n-button quaternary circle size="tiny" @click="removeFolder(i)">
                      <template #icon>
                        <n-icon :component="TrashIcon" size="14" />
                      </template>
                    </n-button>
                  </div>
                </div>

                <!-- 展开内容：模型列表 -->
                <div
                  v-if="expandedFolders.has(item.folder)"
                  class="folder-body"
                >
                  <div v-if="item.models.length === 0" class="no-models">
                    此文件夹下未找到 .gguf 文件
                  </div>
                  <div v-else class="model-grid">
                    <div
                      v-for="m in item.models"
                      :key="m.path"
                      class="model-item"
                    >
                      <div class="model-item-head">
                        <n-icon :component="CubeIcon" size="14" />
                        <span class="model-name" :title="m.file_name">{{ m.file_name }}</span>
                      </div>
                      <div v-if="m.meta" class="model-item-stats">
                        <span class="ms">{{ m.meta.architecture || "—" }}</span>
                        <span class="ms">{{ m.meta.file_type_name || "—" }}</span>
                        <span class="ms">{{ formatBytes(m.file_size_bytes) }}</span>
                        <span v-if="m.meta.context_length" class="ms">
                          ctx {{ Number(m.meta.context_length).toLocaleString() }}
                        </span>
                      </div>
                      <div v-else-if="m.parse_error" class="model-item-err">
                        解析失败：{{ m.parse_error }}
                      </div>
                      <div v-else class="model-item-stats">
                        <span class="ms">{{ formatBytes(m.file_size_bytes) }}</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>
        </div>
      </div>

      <!-- ============ Tab: 启动预设 ============ -->
      <div v-show="activeTab === 'presets'" class="tab-pane preset-pane">
        <div class="preset-layout">
          <!-- 预设列表 -->
          <section class="panel preset-list-panel">
            <div class="panel-head">
              <div class="panel-title">
                <n-icon :component="OptionsIcon" size="18" />
                <span>预设列表</span>
              </div>
              <n-button size="tiny" quaternary @click="newPreset">
                <template #icon>
                  <n-icon :component="AddIcon" />
                </template>
              </n-button>
            </div>
            <div class="preset-items">
              <div
                v-for="p in presetList"
                :key="p.id"
                class="preset-item"
                :class="{ active: editingPreset?.id === p.id }"
                @click="editPreset(p)"
              >
                <div class="preset-item-name">{{ p.name }}</div>
                <div class="preset-item-desc">
                  ctx {{ p.ctx_size || "默认" }} · port {{ p.port }} · ngl {{ p.n_gpu_layers }}
                </div>
                <div class="preset-item-actions" @click.stop>
                  <n-button quaternary circle size="tiny" @click="duplicatePreset(p)">
                    <n-icon :component="CopyIcon" size="14" />
                  </n-button>
                  <n-button
                    quaternary
                    circle
                    size="tiny"
                    :disabled="presetList.length <= 1"
                    @click="deletePreset(p.id)"
                  >
                    <n-icon :component="TrashIcon" size="14" />
                  </n-button>
                </div>
              </div>
            </div>
          </section>

          <!-- 预设编辑器 -->
          <section class="panel preset-editor-panel">
            <div v-if="editingPreset" class="editor-inner">
              <div class="panel-head editor-head">
                <n-input v-model:value="editingPreset.name" size="small" style="width: 220px" />
                <n-button size="small" type="primary" @click="savePreset">保存预设</n-button>
              </div>

              <div class="editor-scroll">
              <n-form label-placement="left" :label-width="120" size="small" class="preset-form">
                <n-divider title-placement="left">基础</n-divider>
                <n-form-item label="上下文长度 (-c)">
                  <n-input-number v-model:value="editingPreset.ctx_size" :min="0" :step="512" />
                  <span class="form-hint">0 = 模型默认</span>
                </n-form-item>
                <n-form-item label="GPU卸载 (-ngl)">
                  <n-input-group>
                    <n-select
                      v-model:value="editingPreset.n_gpu_layers"
                      :options="[
                        { label: 'Auto', value: 'auto' },
                        { label: 'All', value: 'all' },
                        { label: '指定数量', value: 'count' },
                      ]"
                      style="width: 130px"
                    />
                    <n-input-number
                      v-if="editingPreset.n_gpu_layers === 'count'"
                      :value="typeof editingPreset.n_gpu_layers === 'number' ? editingPreset.n_gpu_layers : 0"
                      :min="0"
                      :max="999"
                      @update:value="(v: number | null) => (editingPreset!.n_gpu_layers = v ?? 0)"
                      style="width: 100px"
                    />
                  </n-input-group>
                </n-form-item>
                <n-form-item label="线程数 (-t)">
                  <n-input-number v-model:value="editingPreset.threads" :min="-1" />
                  <span class="form-hint">-1 = 自动</span>
                </n-form-item>
                <n-form-item label="监听地址 (--host)">
                  <n-input v-model:value="editingPreset.host" />
                </n-form-item>
                <n-form-item label="端口 (-p)">
                  <n-input-number v-model:value="editingPreset.port" :min="1" :max="65535" />
                </n-form-item>
                <n-form-item label="别名 (-a)">
                  <n-input v-model:value="editingPreset.alias as any" placeholder="可选" />
                </n-form-item>

                <n-divider title-placement="left">性能</n-divider>
                <n-form-item label="Flash Attention">
                  <n-radio-group v-model:value="editingPreset.flash_attn">
                    <n-radio value="on">On</n-radio>
                    <n-radio value="off">Off</n-radio>
                    <n-radio value="auto">Auto</n-radio>
                  </n-radio-group>
                </n-form-item>
                <n-form-item label="批大小 (-b)">
                  <n-input-number v-model:value="editingPreset.batch_size" :min="1" :step="256" />
                </n-form-item>
                <n-form-item label="微批大小 (-ub)">
                  <n-input-number v-model:value="editingPreset.ubatch_size" :min="1" :step="64" />
                </n-form-item>
                <n-form-item label="并行 (-np)">
                  <n-input-number v-model:value="editingPreset.parallel" :min="1" />
                </n-form-item>

                <n-divider title-placement="left">缓存</n-divider>
                <n-form-item label="K缓存类型">
                  <n-select
                    v-model:value="editingPreset.cache_type_k"
                    :options="['F32','F16','BF16','Q8_0','Q4_0','Q4_1','Q5_0','Q5_1','IQ4_NL'].map(v => ({ label: v, value: v }))"
                  />
                </n-form-item>
                <n-form-item label="V缓存类型">
                  <n-select
                    v-model:value="editingPreset.cache_type_v"
                    :options="['F32','F16','BF16','Q8_0','Q4_0','Q4_1','Q5_0','Q5_1','IQ4_NL'].map(v => ({ label: v, value: v }))"
                  />
                </n-form-item>
                <n-form-item label="连续批处理">
                  <n-switch v-model:value="editingPreset.cont_batching" />
                </n-form-item>

                <n-divider title-placement="left">安全</n-divider>
                <n-form-item label="API密钥">
                  <n-input v-model:value="editingPreset.api_key as any" type="password" placeholder="可选" show-password-on="click" />
                </n-form-item>

                <n-divider title-placement="left">高级</n-divider>
                <n-form-item label="GPU分割模式">
                  <n-select
                    v-model:value="editingPreset.split_mode"
                    :options="[
                      { label: 'None', value: 'none' },
                      { label: 'Layer', value: 'layer' },
                      { label: 'Row', value: 'row' },
                    ]"
                  />
                </n-form-item>
                <n-form-item label="主GPU">
                  <n-input-number v-model:value="editingPreset.main_gpu as any" :min="0" placeholder="可选" />
                </n-form-item>
                <n-form-item label="Jinja模板">
                  <n-switch v-model:value="editingPreset.jinja" />
                </n-form-item>
                <n-form-item label="聊天模板">
                  <n-input v-model:value="editingPreset.chat_template as any" type="textarea" :rows="2" placeholder="可选" />
                </n-form-item>
              </n-form>
              </div>
            </div>
            <div v-else class="editor-empty">
              <n-icon :component="OptionsIcon" size="40" />
              <p>选择左侧预设或点击 + 新建</p>
            </div>
          </section>
        </div>
      </div>
    </main>
  </div>
</template>

<style scoped>
.settings-view {
  height: 100vh;
  display: flex;
  flex-direction: column;
  position: relative;
  overflow: hidden;
}

/* 背景光晕 */
.settings-view::before {
  content: "";
  position: absolute;
  top: -200px;
  right: -100px;
  width: 700px;
  height: 450px;
  background: radial-gradient(ellipse at center, rgba(99, 102, 241, 0.1), transparent 70%);
  pointer-events: none;
  z-index: 0;
}

/* === Header === */
.settings-header {
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
.header-left {
  display: flex;
  align-items: center;
  gap: 8px;
}
.header-right {
  display: flex;
  align-items: center;
  gap: 8px;
}
.header-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-base);
}
.header-hint {
  font-size: 11px;
  color: var(--text-muted);
  margin-left: 6px;
  padding-left: 8px;
  border-left: 1px solid var(--border-strong);
}

/* === 浮动胶囊导航 === */
.tab-nav {
  position: relative;
  z-index: 2;
  flex-shrink: 0;
  display: flex;
  justify-content: center;
  padding: 14px 20px 10px;
}
.tab-nav-inner {
  display: flex;
  gap: 4px;
  padding: 5px;
  border-radius: 999px;
  background: var(--panel-bg);
  border: 1px solid var(--panel-border-strong);
  backdrop-filter: blur(14px) saturate(140%);
  -webkit-backdrop-filter: blur(14px) saturate(140%);
  box-shadow: var(--shadow-card);
  transition: background-color 0.25s ease, border-color 0.25s ease;
}
.tab-btn {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 8px 18px;
  border: none;
  border-radius: 999px;
  background: transparent;
  color: inherit;
  cursor: pointer;
  font-size: 13px;
  opacity: 0.55;
  white-space: nowrap;
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}
.tab-btn:hover {
  opacity: 0.85;
  background: var(--hover-bg);
}
.tab-btn.active {
  opacity: 1;
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.38), rgba(139, 92, 246, 0.38));
  color: var(--accent-text-strong);
  box-shadow:
    0 2px 12px rgba(99, 102, 241, 0.35),
    inset 0 1px 0 rgba(255, 255, 255, 0.12);
}

/* === 主内容 === */
.settings-main {
  position: relative;
  z-index: 1;
  flex: 1;
  min-height: 0; /* flex 子项允许收缩，内部 overflow 才生效 */
  overflow: hidden;
  border-top: 1px solid var(--border-faint);
  margin-top: 4px;
}
.tab-pane {
  height: 100%;
  overflow-y: auto;
  padding: 20px;
  /* 不用 flex 居中：flex 容器作为滚动容器时 scrollHeight 计算有缺陷，
     会导致滚动条触底但内容仍被截断。改用 block + 子元素 margin:auto 居中。 */
}
.container {
  width: 100%;
  max-width: 760px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

/* === Panel === */
.panel {
  padding: 18px 20px;
  border-radius: 14px;
  background: var(--panel-bg);
  border: 1px solid var(--panel-border);
  transition: border-color 0.2s, background-color 0.25s ease;
}
.panel:hover {
  border-color: var(--panel-border-strong);
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
.panel-actions {
  display: flex;
  gap: 8px;
}
.count-tag {
  background: var(--accent-bg);
  color: var(--accent-text);
  font-weight: 500;
}
.ver-tag {
  background: rgba(16, 185, 129, 0.12);
  color: #34d399;
  font-weight: 500;
}

/* === 版本 & 更新 === */
.version-row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 16px;
}
.version-label {
  font-size: 13px;
  opacity: 0.6;
}
.version-empty {
  font-size: 13px;
  opacity: 0.4;
}
.update-section {
  padding-top: 14px;
  border-top: 1px solid var(--border-faint);
}
.update-hint {
  margin-top: 10px;
  font-size: 12px;
  opacity: 0.45;
}
.update-result {
  margin-top: 12px;
}
.update-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  border-radius: 10px;
  border: 1px solid;
}
.update-card.warn {
  background: rgba(245, 158, 11, 0.08);
  border-color: rgba(245, 158, 11, 0.3);
  color: #fbbf24;
}
.update-card.ok {
  background: rgba(16, 185, 129, 0.08);
  border-color: rgba(16, 185, 129, 0.25);
  color: #34d399;
}
.update-card.error {
  background: rgba(239, 68, 68, 0.08);
  border-color: rgba(239, 68, 68, 0.25);
  color: #f87171;
}
.update-card-body {
  flex: 1;
  min-width: 0;
}
.update-card-title {
  font-size: 14px;
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 8px;
}
.update-card-desc {
  font-size: 12px;
  opacity: 0.7;
  margin-top: 2px;
}
.pub-time {
  opacity: 0.6;
}

/* === 模型文件夹 === */
.empty-folders {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
  opacity: 0.45;
  text-align: center;
}
.empty-folders p {
  margin: 10px 0 2px;
  font-size: 14px;
  font-weight: 600;
}
.empty-folders span {
  font-size: 12px;
}

.folder-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.folder-card {
  border-radius: 10px;
  border: 1px solid var(--border-base);
  overflow: hidden;
  transition: border-color 0.15s;
}
.folder-card:hover {
  border-color: rgba(99, 102, 241, 0.25);
}
.folder-head {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 14px;
  cursor: pointer;
  user-select: none;
}
.chevron {
  transition: transform 0.2s;
  opacity: 0.5;
  flex-shrink: 0;
}
.chevron.expanded {
  transform: rotate(180deg);
}
.folder-ico {
  opacity: 0.6;
  flex-shrink: 0;
}
.folder-info {
  flex: 1;
  min-width: 0;
}
.folder-name {
  font-size: 14px;
  font-weight: 600;
}
.folder-path {
  font-size: 11px;
  opacity: 0.45;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-family: ui-monospace, Menlo, Consolas, monospace;
}
.folder-actions {
  display: flex;
  gap: 2px;
}
.folder-body {
  padding: 0 14px 12px;
  border-top: 1px solid var(--border-faint);
}
.no-models {
  padding: 14px;
  text-align: center;
  font-size: 12px;
  opacity: 0.4;
}
.model-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 8px;
  padding-top: 10px;
}
.model-item {
  padding: 10px 12px;
  border-radius: 8px;
  background: var(--surface-bg);
  border: 1px solid var(--surface-border);
  min-width: 0;
  transition: background-color 0.25s ease, border-color 0.25s ease;
}
.model-item-head {
  display: flex;
  align-items: center;
  gap: 6px;
  opacity: 0.6;
  margin-bottom: 6px;
}
.model-name {
  font-size: 13px;
  font-weight: 600;
  opacity: 1.4;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.model-item-stats {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.ms {
  font-size: 11px;
  padding: 2px 7px;
  border-radius: 999px;
  background: var(--accent-bg);
  border: 1px solid var(--accent-border);
  opacity: 0.85;
}
.model-item-err {
  font-size: 11px;
  color: #f87171;
  opacity: 0.8;
}

/* === 预设 === */
.preset-pane {
  overflow: hidden; /* 不再整体滚动，由左右两栏各自管理 */
  display: flex;
  justify-content: center;
  align-items: stretch;
}
.preset-layout {
  width: 100%;
  max-width: 1100px;
  margin: 0 auto;
  display: flex;
  gap: 16px;
  align-items: stretch; /* 左右等高 */
  height: 100%;
}
.preset-list-panel {
  width: 280px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  overflow-y: auto; /* 列表项多时独立滚动 */
  min-height: 0;
}
.preset-items {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.preset-item {
  position: relative;
  padding: 10px 12px;
  border-radius: 10px;
  border: 1px solid var(--border-faint);
  cursor: pointer;
  transition: all 0.15s;
}
.preset-item:hover {
  border-color: rgba(99, 102, 241, 0.25);
  background: rgba(99, 102, 241, 0.04);
}
.preset-item.active {
  border-color: rgba(99, 102, 241, 0.5);
  background: rgba(99, 102, 241, 0.08);
}
.preset-item-name {
  font-size: 13px;
  font-weight: 600;
}
.preset-item-desc {
  font-size: 11px;
  opacity: 0.5;
  margin-top: 3px;
}
.preset-item-actions {
  position: absolute;
  top: 8px;
  right: 8px;
  display: flex;
  gap: 2px;
  opacity: 0;
  transition: opacity 0.15s;
}
.preset-item:hover .preset-item-actions,
.preset-item.active .preset-item-actions {
  opacity: 1;
}

.preset-editor-panel {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden; /* 限制内部滚动区，不让面板本身溢出 */
  min-height: 0;
}
.editor-inner {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.editor-head {
  flex-shrink: 0; /* 顶部"名称+保存"固定不动 */
}
.editor-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto; /* 仅表单区域独立滚动 */
  padding-right: 4px;
}
.editor-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  opacity: 0.35;
}
.editor-empty p {
  font-size: 13px;
}
.preset-form {
  padding-right: 8px;
}

.form-hint {
  font-size: 12px;
  opacity: 0.5;
  margin-left: 8px;
  white-space: nowrap;
}
</style>
