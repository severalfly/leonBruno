<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ApiRequest, ApiResponse, AppSettings, Collection, KeyValue, Workspace } from './types'
import { blankRequest } from './types'

interface Tab { key: string; req: ApiRequest; collection: string; dirty: boolean; response: ApiResponse | null }

const workspace = ref<Workspace>({ collections: [], environments: {} })
const settings = ref<AppSettings>({ workspace_dir: '', active_environment: '', timeout_secs: 30 })
const tabs = ref<Tab[]>([])
const activeKey = ref('')
const activeTab = computed(() => tabs.value.find(t => t.key === activeKey.value) || null)
const req = computed(() => activeTab.value?.req || null)
const response = computed(() => activeTab.value?.response || null)
const sending = ref(false)
const errorMsg = ref('')
const curlText = ref('')
const showCurl = ref(false)
const showSettings = ref(false)
const showEnv = ref(false)
const newCollectionName = ref('')
const sidebarCollapsed = ref(false)
const showImport = ref(false)
const importPath = ref('')
const importProbe = ref('')
const extraRoots = ref<string[]>([])
const probing = ref(false)
const respTab = ref<'body' | 'headers'>('body')
const search = ref('')
const toast = ref('')
let toastTimer: number | undefined

function notify(msg: string) {
  toast.value = msg
  if (toastTimer) clearTimeout(toastTimer)
  toastTimer = window.setTimeout(() => { toast.value = '' }, 2200)
}

async function loadAll() {
  try {
    workspace.value = await invoke<Workspace>('list_workspace')
    settings.value = await invoke<AppSettings>('get_settings')
  } catch (e) { errorMsg.value = String(e) }
}

function openRequest(collection: string, r: ApiRequest) {
  const key = `${collection}::${r.id}`
  let t = tabs.value.find(x => x.key === key)
  if (!t) {
    t = { key, req: JSON.parse(JSON.stringify(r)), collection, dirty: false, response: null }
    tabs.value.push(t)
  }
  activeKey.value = key
  errorMsg.value = ''
}
function newTab(collection?: string) {
  const col = collection || workspace.value.collections[0]?.id || 'Default'
  const r = blankRequest()
  const key = `${col}::${r.id}`
  tabs.value.push({ key, req: r, collection: col, dirty: true, response: null })
  activeKey.value = key
}
function closeTab(key: string) {
  const i = tabs.value.findIndex(t => t.key === key)
  if (i < 0) return
  tabs.value.splice(i, 1)
  if (activeKey.value === key) activeKey.value = tabs.value[Math.max(0, i - 1)]?.key || ''
}
function onCollectionChange(e: Event) {
  if (activeTab.value) activeTab.value.collection = (e.target as HTMLSelectElement).value
}
function updateReq(patch: Partial<ApiRequest>) {
  const t = activeTab.value
  if (!t) return
  Object.assign(t.req, patch)
  t.dirty = true
  t.response = null
}
function updateKv(kind: 'headers' | 'params', i: number, patch: Partial<KeyValue>) {
  const t = activeTab.value
  if (!t) return
  t.req[kind][i] = { ...t.req[kind][i], ...patch }
  t.dirty = true
}
function addKv(kind: 'headers' | 'params') {
  const t = activeTab.value
  if (!t) return
  t.req[kind].push({ key: '', value: '', enabled: true })
}
function removeKv(kind: 'headers' | 'params', i: number) {
  const t = activeTab.value
  if (!t) return
  t.req[kind].splice(i, 1)
  t.dirty = true
}

async function importCurl() {
  if (!curlText.value.trim()) return
  try {
    const parsed = await invoke<ApiRequest>('parse_curl_command', { input: curlText.value })
    const t = activeTab.value
    if (t) {
      t.req = parsed
      t.dirty = true
      t.response = null
    } else {
      const key = `scratch::${parsed.id}`
      tabs.value.push({ key, req: parsed, collection: 'scratch', dirty: true, response: null })
      activeKey.value = key
    }
    showCurl.value = false
    curlText.value = ''
    notify('curl 已导入')
  } catch (e) { errorMsg.value = String(e) }
}

async function save() {
  const t = activeTab.value
  if (!t) return
  if (!t.req.name.trim()) t.req.name = 'New Request'
  try {
    await invoke('save_request', { collection: t.collection, request: t.req })
    t.dirty = false
    await loadAll()
    notify('已保存到本地集合')
  } catch (e) { errorMsg.value = String(e) }
}

async function send() {
  const t = activeTab.value
  if (!t || sending.value) return
  sending.value = true
  errorMsg.value = ''
  t.response = null
  try {
    const env: Record<string, Record<string, string>> = workspace.value.environments || {}
    const active = env[settings.value.active_environment] || {}
    t.response = await invoke<ApiResponse>('send_request', {
      request: t.req,
      env: active,
      timeoutSecs: settings.value.timeout_secs,
    })
    respTab.value = 'body'
  } catch (e) {
    errorMsg.value = String(e)
  } finally { sending.value = false }
}

async function persistSettings() {
  try { await invoke('save_settings', { settings: settings.value }); notify('设置已保存') }
  catch (e) { errorMsg.value = String(e) }
}
async function persistEnv() {
  try { await invoke('save_environments', { envs: workspace.value.environments }); notify('环境变量已保存') }
  catch (e) { errorMsg.value = String(e) }
}
async function addCollection() {
  const n = newCollectionName.value.trim()
  if (!n) return
  try {
    await invoke('create_collection', { name: n })
    newCollectionName.value = ''
    await loadAll()
    notify('集合已创建')
  } catch (e) { errorMsg.value = String(e) }
}
async function loadExtraRoots() {
  try { extraRoots.value = await invoke<string[]>('list_extra_roots') } catch (e) { /* ignore */ }
}
async function probeRepo() {
  importProbe.value = ''
  if (!importPath.value.trim()) return
  probing.value = true
  try {
    const [name, count] = await invoke<[string, number]>('probe_bruno_repo', { path: importPath.value.trim() })
    importProbe.value = `识别到集合「${name}」，含 ${count} 个 .bru/.yml 请求`
  } catch (e) { importProbe.value = `无法识别：${String(e)}` }
  finally { probing.value = false }
}
async function importRepo() {
  if (!importPath.value.trim()) return
  try {
    extraRoots.value = await invoke<string[]>('add_extra_root', { path: importPath.value.trim() })
    await loadAll()
    showImport.value = false
    importPath.value = ''
    importProbe.value = ''
    notify('Bruno 仓库已导入')
  } catch (e) { errorMsg.value = String(e) }
}
async function removeRoot(path: string) {
  try {
    extraRoots.value = await invoke<string[]>('remove_extra_root', { path })
    await loadAll()
    notify('已移除该仓库')
  } catch (e) { errorMsg.value = String(e) }
}

async function openDir() {
  try { await invoke('open_workspace_dir') } catch (e) { errorMsg.value = String(e) }
}
async function removeRequest() {
  const t = activeTab.value
  if (!t || t.req.id.startsWith('r')) return
  if (!confirm(`删除请求「${t.req.name}」？`)) return
  try {
    await invoke('delete_request', { collection: t.collection, id: t.req.id })
    closeTab(t.key)
    await loadAll()
    notify('已删除')
  } catch (e) { errorMsg.value = String(e) }
}

const filteredCollections = computed(() => {
  const q = search.value.trim().toLowerCase()
  if (!q) return workspace.value.collections
  return workspace.value.collections.map(c => ({
    ...c,
    requests: c.requests.filter(r => r.name.toLowerCase().includes(q) || r.url.toLowerCase().includes(q)),
  })).filter(c => c.requests.length > 0 || c.name.toLowerCase().includes(q))
})

onMounted(() => { loadAll(); loadExtraRoots() })
watch(() => settings.value.active_environment, () => { /* 环境切换即时生效，无需额外状态 */ })
</script>

<template>
  <div class="app">
    <header class="titlebar">
      <div class="brand"><span class="logo">LB</span><b>leonBruno</b><span class="tagline">本地优先 · Git 友好的 API 客户端</span></div>
      <div class="env-picker">
        <span>环境</span>
        <select v-model="settings.active_environment">
          <option value="">(无)</option>
          <option v-for="(_, k) in workspace.environments" :key="k" :value="k">{{ k }}</option>
        </select>
      </div>
      <div class="title-actions">
        <button @click="showCurl = !showCurl">粘贴 cURL</button>
        <button @click="showEnv = !showEnv">环境变量</button>
        <button @click="showSettings = !showSettings">设置</button>
      </div>
    </header>

    <div class="body">
      <aside class="sidebar" :class="{ collapsed: sidebarCollapsed }">
        <div v-if="!sidebarCollapsed" class="side-inner">
          <input v-model="search" class="search" placeholder="搜索请求..." />
          <div class="side-section">
            <div class="side-head"><span>集合</span><span><button class="mini" @click="showImport = true">导入仓库</button> <button class="mini" @click="openDir">打开目录</button></span></div>
            <div class="new-col">
              <input v-model="newCollectionName" placeholder="新集合名" @keyup.enter="addCollection" />
              <button class="mini" @click="addCollection">+</button>
            </div>
            <div v-for="c in filteredCollections" :key="c.id" class="collection">
              <div class="col-name">{{ c.name }} <span class="cnt">{{ c.requests.length }}</span></div>
              <div v-for="r in c.requests" :key="r.id" class="req-item" @click="openRequest(c.id, r)">
                <span class="method" :class="r.method">{{ r.method.toUpperCase() }}</span>
                <span class="req-name">{{ r.name }}</span>
              </div>
            </div>
            <div v-if="!filteredCollections.length" class="side-empty">还没有集合<br /><small>点「导入仓库」加载现有 Bruno 集合，或新建集合 / 粘贴 cURL</small></div>
          </div>
        </div>
        <button class="collapse" @click="sidebarCollapsed = !sidebarCollapsed">{{ sidebarCollapsed ? '›' : '‹' }}</button>
      </aside>

      <main class="main">
        <div v-if="tabs.length" class="tabbar">
          <div v-for="t in tabs" :key="t.key" class="tab" :class="{ active: t.key === activeKey }" @click="activeKey = t.key">
            <span class="method" :class="t.req.method">{{ t.req.method.toUpperCase() }}</span>
            <span class="tab-name">{{ t.req.name }}<i v-if="t.dirty">*</i></span>
            <button class="x" @click.stop="closeTab(t.key)">×</button>
          </div>
          <button class="newtab" @click="newTab()">+</button>
        </div>
        <div v-else class="welcome">
          <h2>leonBruno</h2>
          <p>本地优先的 API 客户端 — 请求存在本地文件里，可直接用 Git 管理。</p>
          <div class="welcome-actions">
            <button class="primary" @click="newTab()">新建请求</button>
            <button @click="showCurl = true">从 cURL 导入</button>
          </div>
        </div>

        <template v-if="req">
          <div class="request-bar">
            <select v-model="req.method" class="method-sel" @change="updateReq({ method: req.method })">
              <option v-for="m in ['get','post','put','patch','delete','head','options']" :key="m" :value="m">{{ m.toUpperCase() }}</option>
            </select>
            <input v-model="req.url" class="url" placeholder="https://api.example.com/v1/users (支持 {{变量}})" @input="updateReq({ url: req.url })" @keyup.enter="send" />
            <button class="send" :disabled="sending" @click="send">{{ sending ? '发送中…' : '发送' }}</button>
          </div>

          <div class="editor">
            <div class="req-meta">
              <label>名称 <input v-model="req.name" @input="updateReq({ name: req.name })" /></label>
              <label>集合
                <select :value="activeTab?.collection" @change="onCollectionChange">
                  <option v-for="c in workspace.collections" :key="c.id" :value="c.id">{{ c.name }}</option>
                </select>
              </label>
              <button v-if="!req.id.startsWith('r')" @click="removeRequest">删除请求</button>
            </div>

            <div class="sections">
              <details open>
                <summary>Params <span class="cnt">{{ req.params.filter(p => p.enabled && p.key).length }}</span></summary>
                <table class="kv">
                  <thead><tr><th class="c-chk"></th><th>Key</th><th>Value</th><th></th></tr></thead>
                  <tbody>
                    <tr v-for="(p, i) in req.params" :key="i">
                      <td><input type="checkbox" v-model="p.enabled" @change="updateKv('params', i, { enabled: p.enabled })" /></td>
                      <td><input v-model="p.key" placeholder="key" @input="updateKv('params', i, { key: p.key })" /></td>
                      <td><input v-model="p.value" placeholder="value" @input="updateKv('params', i, { value: p.value })" /></td>
                      <td><button class="del" @click="removeKv('params', i)">×</button></td>
                    </tr>
                  </tbody>
                </table>
                <button class="mini" @click="addKv('params')">+ 添加参数</button>
              </details>

              <details>
                <summary>Headers <span class="cnt">{{ req.headers.filter(h => h.enabled && h.key).length }}</span></summary>
                <table class="kv">
                  <thead><tr><th class="c-chk"></th><th>Key</th><th>Value</th><th></th></tr></thead>
                  <tbody>
                    <tr v-for="(h, i) in req.headers" :key="i">
                      <td><input type="checkbox" v-model="h.enabled" @change="updateKv('headers', i, { enabled: h.enabled })" /></td>
                      <td><input v-model="h.key" placeholder="Header" @input="updateKv('headers', i, { key: h.key })" /></td>
                      <td><input v-model="h.value" placeholder="value" @input="updateKv('headers', i, { value: h.value })" /></td>
                      <td><button class="del" @click="removeKv('headers', i)">×</button></td>
                    </tr>
                  </tbody>
                </table>
                <button class="mini" @click="addKv('headers')">+ 添加 Header</button>
              </details>

              <details>
                <summary>Auth <span class="cnt">{{ req.auth.kind }}</span></summary>
                <div class="auth">
                  <select v-model="req.auth.kind" @change="updateReq({ auth: req.auth })">
                    <option value="none">无</option><option value="bearer">Bearer Token</option><option value="basic">Basic Auth</option>
                  </select>
                  <template v-if="req.auth.kind === 'bearer'">
                    <input v-model="req.auth.token" placeholder="Token (支持 {{变量}})" @input="updateReq({ auth: req.auth })" />
                  </template>
                  <template v-else-if="req.auth.kind === 'basic'">
                    <input v-model="req.auth.username" placeholder="用户名" @input="updateReq({ auth: req.auth })" />
                    <input v-model="req.auth.password" type="password" placeholder="密码" @input="updateReq({ auth: req.auth })" />
                  </template>
                </div>
              </details>

              <details :open="req.body_type !== 'none'">
                <summary>Body <span class="cnt">{{ req.body_type }}</span></summary>
                <div class="body-opts">
                  <label v-for="b in ['none','raw','json','form']" :key="b">
                    <input type="radio" v-model="req.body_type" :value="b" @change="updateReq({ body_type: req.body_type })" /> {{ b }}
                  </label>
                </div>
                <textarea v-if="req.body_type !== 'none'" v-model="req.body" class="body-input" spellcheck="false" placeholder='{"key": "value"}' @input="updateReq({ body: req.body })"></textarea>
              </details>
            </div>
            <div class="save-bar"><button class="primary" @click="save">保存到集合</button><span v-if="activeTab?.dirty" class="dirty">● 未保存</span></div>
          </div>
        </template>
      </main>

      <aside class="response">
        <div v-if="errorMsg" class="error">{{ errorMsg }}</div>
        <template v-if="response">
          <div class="resp-summary">
            <span class="status" :class="{ ok: response.status < 400 }">{{ response.status }} {{ response.status_text }}</span>
            <span>{{ response.elapsed_ms }} ms</span>
            <span>{{ (response.size / 1024).toFixed(1) }} KB</span>
          </div>
          <div class="resp-tabs">
            <button :class="{ active: respTab === 'body' }" @click="respTab = 'body'">Body</button>
            <button :class="{ active: respTab === 'headers' }" @click="respTab = 'headers'">Headers ({{ response.headers.length }})</button>
          </div>
          <pre v-if="respTab === 'body'" class="resp-body">{{ response.body }}</pre>
          <table v-else class="kv resp-headers">
            <tbody>
              <tr v-for="(h, i) in response.headers" :key="i"><td class="hk">{{ h.key }}</td><td>{{ h.value }}</td></tr>
            </tbody>
          </table>
        </template>
        <div v-else class="resp-empty">{{ sending ? '请求中…' : '发送请求后在这里查看响应' }}</div>
      </aside>
    </div>

    <div v-if="showCurl" class="overlay" @click.self="showCurl = false">
      <div class="modal">
        <h3>粘贴 cURL 命令</h3>
        <p class="hint">从浏览器 DevTools → Copy as cURL，或终端直接复制，点解析即可自动识别方法、URL、Header、参数、Body 和认证。</p>
        <textarea v-model="curlText" class="curl-input" spellcheck="false" placeholder="curl -X POST 'https://api.example.com/v1/users' -H 'Authorization: Bearer xxx' -d 'name=leon'"></textarea>
        <div class="modal-actions"><button @click="showCurl = false">取消</button><button class="primary" @click="importCurl">解析并导入</button></div>
      </div>
    </div>

    <div v-if="showEnv" class="overlay" @click.self="showEnv = false">
      <div class="modal wide">
        <h3>环境变量</h3>
        <p class="hint">用双花括号包裹变量名（例如左花括号左花括号 baseUrl 右花括号右花括号）在请求里引用。切换顶部环境即可使用不同取值。env 文件存在本地 workspace 目录。</p>
        <div v-for="(vars, name) in workspace.environments" :key="name" class="env-block">
          <div class="env-name">{{ name }}</div>
          <div v-for="(_, k) in vars" :key="k" class="env-row">
            <input :value="k" disabled />
            <input v-model="vars[k]" @input="persistEnv" />
          </div>
        </div>
        <div class="modal-actions"><button @click="showEnv = false">关闭</button></div>
      </div>
    </div>

    <div v-if="showImport" class="overlay" @click.self="showImport = false">
      <div class="modal">
        <h3>导入现有 Bruno 仓库</h3>
        <p class="hint">填入本机已有的 Bruno 集合目录（含 bruno.json / .bru / .yml 的文件夹）。leonBruno 会<strong>直接读取原目录</strong>，不复制、不改动你的 Git 仓库，Bruno 和 leonBruno 可以同时用同一份集合。</p>
        <input v-model="importPath" class="import-path" placeholder="例如 D://work//my-bruno-api" @keyup.enter="probeRepo" />
        <button class="mini" :disabled="probing" @click="probeRepo">{{ probing ? '识别中…' : '识别目录' }}</button>
        <div v-if="importProbe" class="probe-result" :class="{ ok: importProbe.startsWith('识别到') }">{{ importProbe }}</div>
        <div v-if="extraRoots.length" class="root-list">
          <div class="root-list-title">已导入的仓库</div>
          <div v-for="r in extraRoots" :key="r" class="root-item"><span class="root-path">{{ r }}</span><button class="del" @click="removeRoot(r)">移除</button></div>
        </div>
        <div class="modal-actions"><button @click="showImport = false">取消</button><button class="primary" :disabled="!importProbe.startsWith('识别到')" @click="importRepo">导入</button></div>
      </div>
    </div>

    <div v-if="showSettings" class="overlay" @click.self="showSettings = false">
      <div class="modal">
        <h3>设置</h3>
        <label class="set-row">集合格式<select v-model="settings.format"><option value="bru">.bru（Bru 原生）</option><option value="yaml">.yml（OpenCollection YAML）</option></select></label>
        <label class="set-row">超时（秒）<input v-model.number="settings.timeout_secs" type="number" min="1" max="600" /></label>
        <label class="set-row">当前环境<select v-model="settings.active_environment"><option value="">(无)</option><option v-for="(_, k) in workspace.environments" :key="k" :value="k">{{ k }}</option></select></label>
        <button class="mini" @click="openDir">打开本地集合目录</button>
        <div class="modal-actions"><button @click="showSettings = false">取消</button><button class="primary" @click="persistSettings(); showSettings = false">保存</button></div>
      </div>
    </div>

    <div v-if="toast" class="toast">{{ toast }}</div>
  </div>
</template>
