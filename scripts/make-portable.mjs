import { existsSync, mkdirSync, copyFileSync, writeFileSync, statSync, unlinkSync, readFileSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = dirname(dirname(fileURLToPath(import.meta.url)))
const target = process.env.TAURI_TARGET || 'x86_64-pc-windows-gnu'
const exe = join(root, 'src-tauri', 'target', target, 'release', 'leonbruno.exe')
if (!existsSync(exe)) { console.error('[portable] 找不到 leonbruno.exe'); process.exit(1) }
let version = '0.1.0'
try { version = JSON.parse(readFileSync(join(root, 'src-tauri', 'tauri.conf.json'), 'utf8')).version || version } catch {}
const out = join(root, 'dist-portable')
mkdirSync(out, { recursive: true })
for (const n of ['leonBruno.exe', 'WebView2Loader.dll', 'portable.dat', 'README.txt']) {
  const p = join(out, n); if (existsSync(p)) unlinkSync(p)
}
copyFileSync(exe, join(out, 'leonBruno.exe'))
const dll = join(dirname(exe), 'WebView2Loader.dll')
if (existsSync(dll)) copyFileSync(dll, join(out, 'WebView2Loader.dll'))
writeFileSync(join(out, 'portable.dat'), 'leonBruno portable mode\n')
writeFileSync(join(out, 'README.txt'), `leonBruno ${version} - Windows 便携版
=====================================
【启动】双击 leonBruno.exe（WebView2 运行时需已安装；Win11 自带）
【定位】本地优先、Git 友好的轻量 API 客户端，无需登录、不联网上传任何数据
【cURL】点顶部「粘贴 cURL」，从浏览器 DevTools Copy as cURL 直接粘贴，
  自动识别 method / URL / Header / Query / Body / Bearer / Basic Auth
【标签页】全局扁平标签；切换集合/环境不会重组标签页
【数据】集合以纯文本 JSON 存在 data/workspace/ 下，一个集合一个目录、
  一个请求一个 .json 文件，可直接放进 Git 仓库做版本控制
【变量】支持 {{变量名}} 插值；环境变量存 data/workspace/.env.json
【界面】左侧集合树 → 中间请求编辑 → 右侧响应；顶部切换环境
生成时间：${new Date().toLocaleString('zh-CN')}
`)
console.log(`[portable] 完成 ${out}  leonBruno.exe ${(statSync(join(out, 'leonBruno.exe')).size/1048576).toFixed(2)} MB`)
