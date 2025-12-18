<script setup>
import { ref, onMounted, onUnmounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();
const showSettings = ref(false);
const isPinned = ref(false);

const stats = ref({
  service_req_count: 0,
  service_avg_time: 0,
  db_blocked_count: 0,
  spider_cache_count: 0,
  spider_queue_size: 0,
  spider_fail_count: 0,
  spider_req_avg_time: 0,
  uptime: 0,
  session_cleaned_count: 0,
  is_paused: false,
  spider_total_received: 0,
  spider_actual_reqs: 0,
  server_status: 0, // 0: Init, 1: Running, 2: Failed
});

const config = ref({
  cache_expiration_days: 7,
  proxy_url: "",
  proxy_enabled: false,
  theme: "light"
});

let intervalId = null;

// Helper to format seconds into HH:MM:SS
const formatUptime = (seconds) => {
  const h = Math.floor(seconds / 3600).toString().padStart(2, '0');
  const m = Math.floor((seconds % 3600) / 60).toString().padStart(2, '0');
  const s = (seconds % 60).toString().padStart(2, '0');
  return `${h}:${m}:${s}`;
};

function applyTheme() {
  const root = document.documentElement;
  if (config.value.theme === 'dark') {
    root.setAttribute('data-theme', 'dark');
  } else {
    root.removeAttribute('data-theme');
  }
}

async function updateStats() {
  try {
    stats.value = await invoke("get_stats");
  } catch (error) {
    console.error("Failed to fetch stats:", error);
  }
}

async function loadConfig() {
  try {
    const loaded = await invoke("get_app_config");
    config.value = { ...config.value, ...loaded };
    // Handle None/null from Rust
    if (!config.value.proxy_url) config.value.proxy_url = "";
    if (!config.value.theme) config.value.theme = "light";
    applyTheme();
  } catch (error) {
    console.error("Failed to load config:", error);
  }
}

async function saveConfig() {
  try {
    applyTheme();
    // Convert empty string to null/None logic if needed, 
    // but Rust Option<String> handles Some("") fine or we can send null.
    // For simplicity, let's just send what we have.
    await invoke("set_app_config", { config: config.value });
  } catch (error) {
    console.error("Failed to save config:", error);
  }
}

async function toggleSpider() {
  try {
    stats.value.is_paused = await invoke("toggle_spider_status");
  } catch (error) {
    console.error("Failed to toggle spider:", error);
  }
}

async function togglePin() {
  try {
    const newState = !isPinned.value;
    await invoke("set_always_on_top", { alwaysOnTop: newState });
    isPinned.value = newState;
  } catch (error) {
    console.error("Failed to toggle pin:", error);
  }
}

onMounted(() => {
  updateStats();
  loadConfig();
  intervalId = setInterval(updateStats, 1000);
});

onUnmounted(() => {
  if (intervalId) clearInterval(intervalId);
});

const minimize = () => appWindow.minimize();
const close = () => appWindow.close();
const toggleSettings = () => showSettings.value = !showSettings.value;
</script>

<template>
  <div class="app-layout">
    <!-- Title Bar -->
    <div class="titlebar">
      <!-- Settings Icon -->
      <div class="titlebar-left" :class="{ 'is-active': showSettings }" @click="toggleSettings" :title="showSettings ? '关闭设置' : '设置'">
        <Transition name="icon-transition">
          <svg v-if="!showSettings" class="icon-gear" xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="3"></circle>
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
          </svg>
          <svg v-else class="icon-close-settings" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </Transition>
      </div>
      
      <!-- Drag Region (Spacer) -->
      <div class="titlebar-drag" data-tauri-drag-region></div>

      <div class="titlebar-right">
        <div class="titlebar-button" @click="togglePin" :class="{ 'is-active': isPinned }" :title="isPinned ? '取消置顶' : '窗口置顶'">
          <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" :style="{ transform: isPinned ? 'rotate(45deg)' : 'rotate(0deg)', transition: 'transform 0.2s' }">
             <line x1="12" y1="17" x2="12" y2="22"></line>
             <path d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z"></path>
          </svg>
        </div>
        <div class="titlebar-button" @click="minimize" title="最小化">
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="5" y1="12" x2="19" y2="12"></line>
          </svg>
        </div>
        <div class="titlebar-button close" @click="close" title="关闭">
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </div>
      </div>
    </div>

    <!-- Main Content / Settings -->
    <div class="content-area">
      <Transition name="fade-slide" mode="out-in">
        <!-- Dashboard View -->
        <div v-if="!showSettings" class="dashboard-view" key="dashboard">
          
          <!-- Uptime Display -->
          <div class="uptime-display">
             <svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><polyline points="12 6 12 12 16 14"></polyline></svg>
             <span>{{ formatUptime(stats.uptime) }}</span>
             <div class="port-status-wrapper">
                <span v-if="stats.server_status === 1" class="port-status success" title="API服务运行中">端口: 22332</span>
                <span v-else-if="stats.server_status === 2" class="port-status error" title="端口被占用，API服务启动失败">端口被占用</span>
             </div>
          </div>

          <!-- API Section -->
          <div class="stat-card compact">
            <div class="card-header-row">
                <div class="section-header-group">
                   <svg class="section-icon" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="2" width="20" height="8" rx="2" ry="2"></rect><rect x="2" y="14" width="20" height="8" rx="2" ry="2"></rect><line x1="6" y1="6" x2="6.01" y2="6"></line><line x1="6" y1="18" x2="6.01" y2="18"></line></svg>
                   <span class="section-label">API 服务</span>
                </div>
            </div>
            <div class="card-content-row">
                <div class="stat-item">
                  <div class="stat-val text-primary">{{ stats.service_req_count }}</div>
                  <div class="stat-lbl">总请求</div>
                </div>
                <div class="stat-divider"></div>
                <div class="stat-item">
                  <div class="stat-val">{{ stats.service_avg_time.toFixed(1) }} <span class="unit-text">ms</span></div>
                  <div class="stat-lbl">平均耗时</div>
                </div>
            </div>
          </div>

          <!-- Database Section -->
          <div class="stat-card compact">
            <div class="card-header-row">
                <div class="section-header-group">
                   <svg class="section-icon" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                     <ellipse cx="12" cy="5" rx="9" ry="3"></ellipse>
                     <path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"></path>
                     <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"></path>
                   </svg>
                   <span class="section-label">数据库</span>
                </div>
            </div>
            <div class="card-content-row">
                <div class="stat-item">
                  <div class="stat-val text-success">{{ stats.db_blocked_count }}</div>
                  <div class="stat-lbl">已屏蔽用户</div>
                </div>
                <div class="stat-divider"></div>
                <div class="stat-item">
                  <div class="stat-val">{{ stats.spider_cache_count }}</div>
                  <div class="stat-lbl">BV缓存</div>
                </div>
            </div>
          </div>

          <!-- Spider Section -->
          <div class="stat-card">
            <div class="card-header-row">
                <div class="section-header-group">
                   <svg class="section-icon" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                     <circle cx="12" cy="12" r="5"></circle>
                     <path d="M12 7V4"></path>
                     <path d="M16 10l4-2"></path>
                     <path d="M17 12l4 1"></path>
                     <path d="M16 14l4 3"></path>
                     <path d="M8 10L4 8"></path>
                     <path d="M7 12L3 13"></path>
                     <path d="M8 14L4 17"></path>
                   </svg>
                   <span class="section-label">爬虫状态</span>
                </div>
                <button class="spider-toggle-btn" @click="toggleSpider" :class="{ 'is-paused': stats.is_paused }" :title="stats.is_paused ? '点击恢复' : '点击暂停'">
                  <span class="status-dot"></span>
                  {{ stats.is_paused ? '已暂停' : '运行中' }}
                </button>
            </div>
            
            <div class="stat-list">
                <div class="stat-row">
                  <span class="stat-lbl-list">接收查询BV数</span>
                  <span class="stat-val-list text-primary">{{ stats.spider_total_received }}</span>
                </div>
                <div class="stat-row">
                  <span class="stat-lbl-list">请求次数</span>
                  <span class="stat-val-list text-info">{{ stats.spider_actual_reqs }}</span>
                </div>
                <div class="stat-row">
                  <span class="stat-lbl-list">等待队列</span>
                  <span class="stat-val-list" :class="{ 'text-warn': stats.spider_queue_size > 50 }">{{ stats.spider_queue_size }}</span>
                </div>
                <div class="stat-row">
                  <span class="stat-lbl-list">失败请求</span>
                  <span class="stat-val-list" :class="{ 'text-error': stats.spider_fail_count > 0 }">{{ stats.spider_fail_count }}</span>
                </div>
                <div class="stat-row">
                  <span class="stat-lbl-list">平均耗时</span>
                  <span class="stat-val-list">{{ stats.spider_req_avg_time.toFixed(0) }} <span class="unit-text">ms</span></span>
                </div>
            </div>
          </div>

        </div>

        <!-- Settings View -->
        <div v-else class="settings-view" key="settings">
          <div class="settings-header">
            <h2>设置</h2>
          </div>
          <div class="settings-content">
            <div class="setting-item">
              <div class="setting-label">
                <label>界面主题</label>
                <span class="setting-desc">切换日间/夜间模式</span>
              </div>
              <div class="theme-toggle">
                <button 
                  class="theme-btn" 
                  :class="{ active: config.theme === 'light' }" 
                  @click="config.theme = 'light'; saveConfig()"
                  title="日间模式"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="5"></circle><line x1="12" y1="1" x2="12" y2="3"></line><line x1="12" y1="21" x2="12" y2="23"></line><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"></line><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"></line><line x1="1" y1="12" x2="3" y2="12"></line><line x1="21" y1="12" x2="23" y2="12"></line><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"></line><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"></line></svg>
                </button>
                <button 
                  class="theme-btn" 
                  :class="{ active: config.theme === 'dark' }" 
                  @click="config.theme = 'dark'; saveConfig()"
                  title="夜间模式"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path></svg>
                </button>
              </div>
            </div>

            <div class="setting-item">
              <div class="setting-label">
                <label>BV 缓存过期时间</label>
                <span class="setting-desc">超过此时间的 BV 记录将被清除</span>
              </div>
              <div class="setting-input-wrapper">
                <input type="number" v-model.number="config.cache_expiration_days" @change="saveConfig" min="1" />
                <span class="unit">天</span>
              </div>
            </div>

            <div class="setting-item">
              <div class="setting-label">
                <label>启用 HTTP 代理</label>
                <span class="setting-desc">是否使用代理服务器进行爬取</span>
              </div>
              <label class="switch">
                  <input type="checkbox" v-model="config.proxy_enabled" @change="saveConfig">
                  <span class="slider round"></span>
              </label>
            </div>

            <div class="setting-item">
              <div class="setting-label">
                <label>代理地址</label>
                <span class="setting-desc">例如: http://127.0.0.1:7890</span>
              </div>
              <div class="setting-input-wrapper" style="flex: 1; max-width: 200px;">
                <input type="text" v-model="config.proxy_url" @change="saveConfig" placeholder="http://..." style="width: 100%; text-align: left;" :disabled="!config.proxy_enabled" />
              </div>
            </div>
          </div>
        </div>
      </Transition>
    </div>
  </div>
</template>

<style>
/* Global Reset & Base */
:root {
  /* Colors - Light Theme (Default) */
  --bg-app: #f6f6f6;
  --bg-surface: #ffffff;
  --bg-titlebar: #ffffff;
  
  --text-primary: #333333;
  --text-secondary: #666666;
  --text-muted: #999999;
  
  --border-color: #eeeeee;
  --border-strong: #dddddd;
  
  --hover-bg: rgba(0, 0, 0, 0.05);
  
  --accent-color: #007bff;
  --success-color: #28a745;
  --warn-color: #ffc107;
  --error-color: #dc3545;
  --info-color: #17a2b8;

  --titlebar-height: 32px;
}

[data-theme="dark"] {
  /* Colors - Dark Theme */
  --bg-app: #050505; /* Pure black frosted glass feel */
  --bg-surface: #141414;
  --bg-titlebar: #141414;
  
  --text-primary: #e0e0e0;
  --text-secondary: #a0a0a0;
  --text-muted: #666666;
  
  --border-color: #2a2a2a;
  --border-strong: #333333;
  
  --hover-bg: rgba(255, 255, 255, 0.08);
  
  /* Adjust accents for dark mode visibility */
  --accent-color: #3b9eff; 
  --success-color: #3dd660;
  --warn-color: #ffca2c;
  --error-color: #ff6b6b;
  --info-color: #4dd0e1;
}

body {
  margin: 0;
  padding: 0;
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  border-radius: 8px;
  background: var(--bg-surface);
  color: var(--text-primary);
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  font-size: 13px;
  user-select: none;
  cursor: default;
}

#app {
  width: 100%;
  height: 100%;
}
</style>

<style scoped>
/* Transitions */
.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: all 0.25s ease;
}

.fade-slide-enter-from {
  opacity: 0;
  transform: translateY(10px);
}

.fade-slide-leave-to {
  opacity: 0;
  transform: translateY(-10px);
}


/* Layout */
.app-layout {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-surface);
  border: 1px solid var(--border-strong);
  box-sizing: border-box;
  transition: background-color 0.3s, border-color 0.3s;
}


/* Titlebar */
.titlebar {
  height: var(--titlebar-height);
  background: var(--bg-titlebar);
  display: flex;
  justify-content: space-between;
  align-items: center;
  user-select: none;
  flex-shrink: 0;
  transition: background-color 0.3s;
}

.titlebar-left, .titlebar-button {
  width: 40px;
  height: 100%;
  display: flex;
  justify-content: center;
  align-items: center;
  cursor: pointer;
  color: var(--text-secondary);
  transition: color 0.2s, background-color 0.2s;
  position: relative;
}

.uptime-display {
  display: flex;
  align-items: center;
  gap: 6px;
  font-family: 'Consolas', monospace;
  font-size: 11px;
  color: var(--text-muted);
  margin-bottom: 2px;
  user-select: none;
}

.port-status-wrapper {
  margin-left: auto;
  display: flex;
  align-items: center;
}

.port-status {
  padding: 1px 4px;
  border-radius: 4px;
  font-size: 10px;
  font-weight: 600;
}

.port-status.success {
  color: var(--success-color);
  background: rgba(40, 167, 69, 0.1);
}

.port-status.error {
  color: var(--error-color);
  background: rgba(220, 53, 69, 0.1);
}

.titlebar-left:hover, .titlebar-button:hover {
  background: transparent;
  color: var(--accent-color);
}
.titlebar-left.is-active:hover, .titlebar-button.close:hover {
  color: var(--error-color);
}

.icon-transition-enter-active,
.icon-transition-leave-active {
  transition: all 0.3s cubic-bezier(0.4, 0.0, 0.2, 1);
  position: absolute;
}
.icon-transition-enter-from { opacity: 0; transform: rotate(-90deg) scale(0.8); }
.icon-transition-leave-to { opacity: 0; transform: rotate(90deg) scale(0.8); }

.titlebar-drag {
  flex-grow: 1;
  height: 100%;
}

.titlebar-right {
  display: flex;
  height: 100%;
}

/* Content Area */
.content-area {
  flex: 1;
  overflow: hidden;
  position: relative;
  display: flex;
  flex-direction: column;
}

/* Dashboard Styles */
.dashboard-view {
  padding: 8px 10px;
  display: flex;
  flex-direction: column;
  gap: 6px; 
  height: 100%;
  box-sizing: border-box;
}

.stat-card {
  flex: 1;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  padding: 8px 10px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  background: var(--bg-surface);
  justify-content: center;
  transition: background-color 0.3s, border-color 0.3s;
}

.stat-card.compact {
  flex: 0 0 auto;
  padding: 6px 10px;
}
.stat-card.compact .card-header-row {
  margin-bottom: 2px;
}
.stat-card.compact .stat-val {
  font-size: 14px;
}

.card-header-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.section-header-group {
  display: flex;
  align-items: center;
  gap: 6px;
}

.section-icon {
  color: var(--text-muted); /* Was #bbb */
}

.section-label {
  font-size: 11px;
  font-weight: 700;
  color: var(--text-muted); /* Was #999 */
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.uptime-badge {
  display: none;
}

.card-content-row {
  display: flex;
  justify-content: space-around; /* Distribute evenly */
  align-items: center;
  width: 100%;
}

.stat-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  flex: 1;
}

.stat-val {
  font-family: 'Consolas', monospace;
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
  line-height: 1.2;
}

.stat-lbl {
  font-size: 10px;
  color: var(--text-secondary); /* Was #aaa */
  margin-top: 1px;
}

.stat-divider {
  width: 1px;
  height: 16px;
  background: var(--border-color); /* Was #f0f0f0 */
  margin: 0 4px;
}

/* New List Layout Styles */
.stat-list {
  display: grid;
  grid-template-columns: 1fr 1fr;
  column-gap: 16px;
  row-gap: 4px;
}

.stat-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 12px;
  line-height: 1.4;
}

.stat-lbl-list {
  color: var(--text-secondary); /* Was #666 */
  font-size: 11px;
}

.stat-val-list {
  font-family: 'Consolas', monospace;
  font-weight: 600;
  color: var(--text-primary);
  font-size: 12px;
}

.unit-text {
  font-size: 11px;
  color: var(--text-muted);
  font-weight: normal;
}

/* Colors */
.text-primary { color: var(--accent-color); }
.text-success { color: var(--success-color); }
.text-warn { color: var(--warn-color); }
.text-error { color: var(--error-color); }
.text-info { color: var(--info-color); }

/* Spider Toggle Button */
.spider-toggle-btn {
  border: none;
  background: transparent;
  padding: 2px 6px;
  font-size: 11px;
  cursor: pointer;
  border-radius: 10px;
  display: flex;
  align-items: center;
  gap: 4px;
  transition: all 0.2s;
  color: var(--success-color);
  font-weight: 500;
  border: 1px solid rgba(40, 167, 69, 0.2);
}

.spider-toggle-btn:hover {
  background: transparent;
}

.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background-color: var(--success-color);
  box-shadow: 0 0 4px rgba(40, 167, 69, 0.4);
}

.spider-toggle-btn.is-paused {
  color: var(--warn-color); /* Darker yellow */
  border-color: rgba(255, 193, 7, 0.3);
}
.spider-toggle-btn.is-paused:hover {
  background: transparent;
}
.spider-toggle-btn.is-paused .status-dot {
  background-color: var(--warn-color);
  box-shadow: none;
}


/* Settings View Styles (Preserved) */
.settings-view {
  padding: 0 16px;
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow-y: auto;
  scrollbar-width: none; /* Firefox */
}
.settings-view::-webkit-scrollbar {
  display: none; /* Chrome, Safari, Edge */
}

.settings-header {
  padding: 16px 0 12px 0;
  margin-bottom: 0;
}

.settings-header h2 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  letter-spacing: 0.5px;
}

.settings-content {
  display: flex;
  flex-direction: column;
}

.setting-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 0;
  background: transparent;
  gap: 12px;
}

.setting-item:last-child {
  border-bottom: none;
}

.setting-label {
  display: flex;
  flex-direction: column;
  flex: 1;
  text-align: left;
  overflow: hidden; 
}

.setting-label label {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
  margin-bottom: 4px;
}

.setting-desc {
  font-size: 12px;
  color: var(--text-muted);
  line-height: 1.4;
}

.setting-input-wrapper {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.setting-input-wrapper input {
  width: 48px;
  padding: 4px 8px;
  border: 1px solid var(--border-strong);
  background: transparent;
  text-align: center;
  font-family: inherit;
  font-size: 13px;
  color: var(--text-primary);
  transition: border-color 0.2s;
  border-radius: 4px;
}

.setting-input-wrapper input:focus {
  outline: none;
  border-color: var(--accent-color);
}
.setting-input-wrapper input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

input[type=number]::-webkit-inner-spin-button, 
input[type=number]::-webkit-outer-spin-button { 
  -webkit-appearance: none; 
  margin: 0; 
}
input[type=number] {
  -moz-appearance: textfield;
}

.unit {
  font-size: 12px;
  color: var(--text-muted);
}

/* Theme Toggle Buttons */
.theme-toggle {
  display: flex;
  background: var(--border-color);
  padding: 2px;
  border-radius: 6px;
  gap: 2px;
}

.theme-btn {
  border: none;
  background: transparent;
  width: 28px;
  height: 24px;
  border-radius: 4px;
  cursor: pointer;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
}

.theme-btn:hover {
  color: var(--text-primary);
}

.theme-btn.active {
  background: var(--bg-surface);
  color: var(--accent-color);
  box-shadow: 0 1px 2px rgba(0,0,0,0.1);
}

/* Switch Toggle */
.switch {
  position: relative;
  display: inline-block;
  width: 32px;
  height: 18px;
  margin-right: 8px;
}

.switch input { 
  opacity: 0;
  width: 0;
  height: 0;
}

.slider {
  position: absolute;
  cursor: pointer;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: #ccc; /* Keep hardcoded gray for off state or use variable */
  transition: .4s;
  border-radius: 34px;
}

.slider:before {
  position: absolute;
  content: "";
  height: 14px;
  width: 14px;
  left: 2px;
  bottom: 2px;
  background-color: white;
  transition: .4s;
  border-radius: 50%;
}

input:checked + .slider {
  background-color: var(--accent-color);
}

input:focus + .slider {
  box-shadow: 0 0 1px var(--accent-color);
}

input:checked + .slider:before {
  transform: translateX(14px);
}
</style>