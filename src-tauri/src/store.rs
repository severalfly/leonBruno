use crate::types::{ApiRequest, AppSettings, Collection, Workspace};
use std::{fs, path::{Path, PathBuf}};
use tauri::Manager;

pub fn data_dir(app: &tauri::AppHandle) -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            if dir.join("portable.dat").exists() { return dir.join("data"); }
        }
    }
    app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// 集合根目录：Bruno 兼容布局，可直接被 Bruno 打开。
/// data/workspace/<集合名>/{bruno.json, collection.bru, *.bru|*.yml, environments/*.bru}
pub fn workspace_dir(app: &tauri::AppHandle) -> PathBuf {
    let dir = data_dir(app).join("workspace");
    let _ = fs::create_dir_all(&dir);
    dir
}
fn settings_path(app: &tauri::AppHandle) -> PathBuf { data_dir(app).join("settings.json") }
fn extra_roots_path(app: &tauri::AppHandle) -> PathBuf { data_dir(app).join("extra_roots.json") }

/// 导入的外部 Bruno 仓库根目录列表（不复制文件，直接读取原目录）
pub fn load_extra_roots(app: &tauri::AppHandle) -> Vec<String> {
    fs::read_to_string(extra_roots_path(app)).ok()
        .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}
pub fn save_extra_roots(app: &tauri::AppHandle, roots: &[String]) -> Result<(), String> {
    fs::write(extra_roots_path(app), serde_json::to_string_pretty(roots).map_err(|e|e.to_string())?)
        .map_err(|e|e.to_string())
}
/// 判断一个目录是不是 Bruno 集合（三种格式：OpenCollection YAML / 传统 .bru / 混合）
fn is_collection_dir(p: &Path) -> bool {
    p.join("opencollection.yml").exists() || p.join("opencollection.yaml").exists()
        || p.join("bruno.json").exists() || p.join("collection.bru").exists()
}
/// 读集合名：优先 opencollection.yml 的 info.name，其次 bruno.json 的 name，最后目录名
fn collection_name(p: &Path) -> String {
    for f in ["opencollection.yml", "opencollection.yaml"] {
        if let Ok(txt)=fs::read_to_string(p.join(f)) {
            // 极简 YAML 读取 info: 下的 name:
            let mut in_info=false;
            for line in txt.lines() {
                let t=line.trim();
                if t=="info:" { in_info=true; continue; }
                if !t.is_empty() && !t.starts_with(' ') { in_info=false; }
                if in_info && t.starts_with("name:") {
                    let v=t[5..].trim().trim_matches(|c| c=='\'' || c=='"');
                    if !v.is_empty() { return v.to_string(); }
                }
            }
        }
    }
    if let Ok(txt)=fs::read_to_string(p.join("bruno.json")) {
        if let Ok(v)=serde_json::from_str::<serde_json::Value>(&txt) {
            if let Some(n)=v.get("name").and_then(|x|x.as_str()) { return n.to_string(); }
        }
    }
    p.file_name().and_then(|n|n.to_str()).unwrap_or("collection").to_string()
}

/// 校验一个目录是否是 Bruno 集合（含 bruno.json 或 collection.bru 或 .bru/.yml 请求）
pub fn probe_bruno_repo(path: &str) -> Result<(String, usize), String> {
    let p = PathBuf::from(path);
    if !p.is_dir() { return Err("目录不存在".into()); }
    let name = collection_name(&p);
    // 情况 A：本目录本身就是一个集合（bruno.json / collection.bru / opencollection.yml / *.bru|*.yml）
    let direct = p.join("bruno.json").exists() || p.join("collection.bru").exists()
        || p.join("opencollection.yml").exists() ||
        fs::read_dir(&p).ok().map(|rd| rd.flatten().any(|e| {
            let fp = e.path();
            let ext=fp.extension().and_then(|x|x.to_str()).unwrap_or("");
            ext=="bru"||ext=="yml"||ext=="yaml"
        })).unwrap_or(false);
    if direct {
        let mut reqs=Vec::new();
        collect_requests(&p, &p, &mut reqs);
        return Ok((name, reqs.len()));
    }
    // 情况 B：本目录是仓库根，子目录里各是集合
    let mut cnt=0usize;
    if let Ok(rd)=fs::read_dir(&p) {
        for e in rd.flatten() {
            if !e.path().is_dir() { continue; }
            if is_collection_dir(&e.path()) { cnt+=1; }
        }
    }
    if cnt>0 { return Ok((name, cnt)); }
    Err("目录里没有找到 Bruno 集合（需要 opencollection.yml / bruno.json / collection.bru / .bru 或 .yml 文件）".into())
}

pub fn load_settings(app: &tauri::AppHandle) -> AppSettings {
    fs::read_to_string(settings_path(app)).ok()
        .and_then(|s| serde_json::from_str::<AppSettings>(&s).ok())
        .unwrap_or_default()
}
pub fn save_settings(app: &tauri::AppHandle, s: &AppSettings) -> Result<(), String> {
    fs::write(settings_path(app), serde_json::to_string_pretty(s).map_err(|e|e.to_string())?)
        .map_err(|e|e.to_string())
}

fn safe_name(s:&str)->String {
    let v: String = s.chars().map(|c| match c { '\\'|'/'|':'|'*'|'?'|'"'|'<'|'>'|'|' => '_', _ => c }).collect();
    let v = v.trim().trim_matches('.').to_string();
    if v.is_empty() { "collection".into() } else { v }
}

fn parse_request_file(path:&Path)->Option<ApiRequest> {
    let txt = fs::read_to_string(path).ok()?;
    let stem = path.file_stem()?.to_str()?.to_string();
    let r = match path.extension().and_then(|e|e.to_str()).unwrap_or("").to_lowercase().as_str() {
        "bru" => crate::bru::parse(&txt, &stem).ok()?,
        "yml"|"yaml" => crate::yamlreq::parse(&txt, &stem).ok()?,
        _ => return None,
    };
    Some(r)
}

/// 读取全部集合。兼容三种源文件：.bru（Bru）、.yml（OpenCollection）、
/// .json（0.1.0 旧格式，读后自动转成 .bru 并删除）。
pub fn load_workspace(app: &tauri::AppHandle) -> Workspace {
    let root = workspace_dir(app);
    let mut ws = Workspace::default();
    // 先扫描内部工作区，再扫描导入的外部 Bruno 仓库（不复制，直接读）
    let mut roots: Vec<PathBuf> = vec![root.clone()];
    for r in load_extra_roots(app) {
        let p = PathBuf::from(&r);
        if p.is_dir() { roots.push(p); }
    }
    for root in roots { scan_root(&root, &mut ws); }
    load_env(&root, &mut ws);
    ws.collections.sort_by(|a,b| a.name.cmp(&b.name));
    return ws;
}

/// 递归收集集合目录下所有请求文件（含子目录分组）。
/// 请求 id 用「相对路径/文件名」保证跨子目录唯一；name 保持文件里的原值。
fn collect_requests(base: &Path, dir: &Path, out: &mut Vec<ApiRequest>) {
    let Ok(entries)=fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let fp=e.path();
        if fp.is_dir() {
            // 跳过环境目录与版本控制/依赖目录
            let n=fp.file_name().and_then(|x|x.to_str()).unwrap_or("");
            if n=="environments"||n==".git"||n=="node_modules" { continue; }
            collect_requests(base, &fp, out);
            continue;
        }
        let ext=fp.extension().and_then(|x|x.to_str()).unwrap_or("").to_lowercase();
        // opencollection.yml / folder.yml 是定义文件，不是请求
        let stem=fp.file_stem().and_then(|x|x.to_str()).unwrap_or("");
        if stem=="opencollection"||stem=="folder" { continue; }
        if ext=="json" && fp.file_name().and_then(|n|n.to_str())!=Some("bruno.json") {
            if let Ok(txt)=fs::read_to_string(&fp) {
                if let Ok(mut r)=serde_json::from_str::<ApiRequest>(&txt) {
                    let rel=fp.strip_prefix(base).unwrap_or(&fp);
                    r.id=rel.to_string_lossy().replace('\\',"/").trim_end_matches(".json").to_string();
                    let bru=crate::bru::stringify(&r);
                    let _=fs::write(fp.with_extension("bru"),bru);
                    let _=fs::remove_file(&fp);
                    r.body_type = if r.body.is_empty(){"none".into()}else{r.body_type.clone()};
                    out.push(r);
                }
            }
            continue;
        }
        if ext=="yml"||ext=="yaml"||ext=="bru" {
            if let Some(mut r)=parse_request_file(&fp) {
                // 子目录里的请求：id 加相对路径前缀，避免同名冲突
                let rel=fp.strip_prefix(base).unwrap_or(&fp);
                let rels=rel.to_string_lossy().replace('\\',"/");
                if rels.contains('/') { r.id=rels.trim_end_matches(&format!(".{}",ext)).to_string(); }
                out.push(r);
            }
        }
    }
}

/// 扫描一个根目录下的一层集合目录
fn scan_root(root: &Path, ws: &mut Workspace) {
    if let Ok(entries) = fs::read_dir(&root) {
        for e in entries.flatten() {
            let p=e.path();
            if !p.is_dir() || p.file_name().and_then(|n|n.to_str()).map(|n|n.starts_with('.')).unwrap_or(false) { continue; }
            let dir_name = match p.file_name().and_then(|n|n.to_str()) { Some(n)=>n.to_string(), None=>continue };
            // 集合显示名：opencollection.yml 的 info.name / bruno.json 的 name / 目录名
            let name = collection_name(&p);
            let mut requests=Vec::new();
            // 递归读取集合目录下所有层级的请求（Bruno 用子目录做分组）
            collect_requests(&p, &p, &mut requests);
            requests.sort_by(|a,b| a.seq.cmp(&b.seq).then_with(||a.name.cmp(&b.name)));
            // 集合自带 environments/*.bru：以文件名（去掉括号/空格）作为环境名合并
            let edir = p.join("environments");
            if let Ok(efs)=fs::read_dir(&edir) {
                for ef in efs.flatten() {
                    let ep=ef.path();
                    if ep.extension().and_then(|x|x.to_str())!=Some("bru") { continue; }
                    let env_name=ep.file_stem().and_then(|x|x.to_str()).unwrap_or("default").to_string();
                    if let Ok(txt)=fs::read_to_string(&ep) {
                        let mut vars=std::collections::BTreeMap::new();
                        let mut in_vars=false;
                        for line in txt.lines() {
                            let t=line.trim();
                            if t=="vars {" { in_vars=true; continue; }
                            if t=="}" { in_vars=false; continue; }
                            if in_vars { if let Some(q)=t.find(':') {
                                let k=t[..q].trim(); let v=t[q+1..].trim().trim_matches(|c| c == '\'' || c == '"');
                                if !k.is_empty() { vars.insert(k.to_string(),v.to_string()); }
                            }}
                        }
                        if !vars.is_empty() { ws.environments.entry(env_name).or_insert(vars); }
                    }
                }
            }
            ws.collections.push(Collection { id: dir_name, name, requests });
        }
    }
}

/// 读环境变量：Bruno environments/*.bru 的 vars{} + 兼容 0.1.0 .env.json
fn load_env(root: &Path, ws: &mut Workspace) {
    let mut envs = std::collections::BTreeMap::new();
    let edir = root.join("environments");
    if let Ok(files)=fs::read_dir(&edir) {
        for f in files.flatten() {
            let fp=f.path();
            if fp.extension().and_then(|x|x.to_str())!=Some("bru") { continue; }
            let name=fp.file_stem().and_then(|s|s.to_str()).unwrap_or("default").to_string();
            let mut vars=std::collections::BTreeMap::new();
            if let Ok(txt)=fs::read_to_string(&fp) {
                let mut in_vars=false; let mut in_secret=false;
                for line in txt.lines() {
                    let t=line.trim();
                    if t=="vars {" { in_vars=true; in_secret=false; continue; }
                    if t=="vars:secret {" || t.starts_with("vars:secret [") { in_vars=false; in_secret=true; continue; }
                    if t=="}" { in_vars=false; in_secret=false; continue; }
                    if !in_vars { continue; }
                    if let Some(p)=t.find(':') {
                        let k=t[..p].trim(); let v=t[p+1..].trim().trim_matches(|c|c=='\''||c=='"');
                        if !k.is_empty() { vars.insert(k.to_string(),v.to_string()); }
                    }
                }
                let _=in_secret;
            }
            envs.insert(name,vars);
        }
    }
    // 兼容 0.1.0 的 .env.json
    if envs.is_empty() {
        if let Ok(txt)=fs::read_to_string(root.join(".env.json")) {
            if let Ok(v)=serde_json::from_str(&txt) { envs=v; }
        }
    }
    ws.environments=envs;
}

/// 写集合元数据（Bruno 需要 bruno.json 才认这是集合）
fn ensure_collection(app:&tauri::AppHandle, id:&str)->Result<PathBuf,String> {
    let dir=workspace_dir(app).join(safe_name(id));
    fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
    let cfg=dir.join("bruno.json");
    if !cfg.exists() {
        let v=serde_json::json!({"version":"1","name":safe_name(id),"type":"collection","ignore":["node_modules",".git",".env"]});
        fs::write(cfg,serde_json::to_string_pretty(&v).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    }
    Ok(dir)
}

pub fn save_request(app:&tauri::AppHandle, collection:&str, req:&ApiRequest)->Result<(),String> {
    let dir=ensure_collection(app,collection)?;
    let id=safe_name(&req.id);
    let fmt=load_settings(app).format;
    // 切换格式或旧格式残留时清理旧文件，避免同名双份
    let _=fs::remove_file(dir.join(format!("{}.bru",id)));
    let _=fs::remove_file(dir.join(format!("{}.yml",id)));
    let _=fs::remove_file(dir.join(format!("{}.json",id)));
    let text=if fmt=="yaml" { crate::yamlreq::stringify(req) } else { crate::bru::stringify(req) };
    let ext=if fmt=="yaml" {"yml"}else{"bru"};
    fs::write(dir.join(format!("{}.{}",id,ext)),text).map_err(|e|e.to_string())
}

pub fn delete_request(app:&tauri::AppHandle, collection:&str, id:&str)->Result<(),String> {
    let dir=workspace_dir(app).join(safe_name(collection));
    for ext in ["bru","yml","yaml","json"] {
        let p=dir.join(format!("{}.{}",safe_name(id),ext));
        if p.exists() { fs::remove_file(p).map_err(|e|e.to_string())?; }
    }
    Ok(())
}

pub fn create_collection(app:&tauri::AppHandle, name:&str)->Result<(),String> {
    let dir=ensure_collection(app,name)?;
    let c=dir.join("collection.bru");
    if !c.exists() { fs::write(c,"meta {\n  type: collection\n}\n").map_err(|e|e.to_string())?; }
    Ok(())
}

pub fn save_environments(app:&tauri::AppHandle, envs:&std::collections::BTreeMap<String,std::collections::BTreeMap<String,String>>)->Result<(),String> {
    let root=workspace_dir(app);
    let dir=root.join("environments");
    fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
    // 保留原有 vars:secret 名单，Secret 不写明文
    for (name,vars) in envs {
        let p=dir.join(format!("{}.bru",safe_name(name)));
        let mut s=String::from("vars {\n");
        for (k,v) in vars { s.push_str(&format!("  {}: {}\n",k,v)); }
        s.push_str("}\n");
        fs::write(p,s).map_err(|e|e.to_string())?;
    }
    Ok(())
}

pub fn open_workspace_dir(app:&tauri::AppHandle)->Result<(),String> {
    let dir=workspace_dir(app);
    #[cfg(target_os="windows")]
    std::process::Command::new("explorer.exe").arg(&dir).spawn().map_err(|e|e.to_string())?;
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_bruno_files() {
        let dir=std::env::temp_dir().join("lb-bruno-fixture");
        let _=fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("environments")).unwrap();
        fs::write(dir.join("bruno.json"),r#"{"version":"1","name":"My API Collection","type":"collection"}"#).unwrap();
        fs::write(dir.join("get-users.bru"),r#"meta {
  name: Get Users
  type: http
  seq: 1
}
get {
  url: {{baseUrl}}/users
  body: none
  auth: none
}
params:query {
  page: 2
  ~debug: true
}
headers {
  Accept: application/json
}
"#).unwrap();
        let r=parse_request_file(&dir.join("get-users.bru")).unwrap();
        assert_eq!(r.name,"Get Users");
        assert_eq!(r.method,crate::types::HttpMethod::Get);
        assert_eq!(r.params.len(),2);
        assert!(r.params[0].enabled);
        assert!(!r.params[1].enabled);
        assert_eq!(r.headers[0].key,"Accept");
    }
    #[test]
    fn parse_yaml_request() {
        let dir=std::env::temp_dir().join("lb-yaml-fixture");
        let _=fs::remove_dir_all(&dir); fs::create_dir_all(&dir).unwrap();
        let p=dir.join("get-profile.yml");
        fs::write(&p,r#"info:
  name: Get Profile
  type: http
  seq: 3
http:
  method: GET
  url: https://api.example.com/profile
  headers:
    - name: Accept
      value: application/json
settings:
  encodeUrl: true
"#).unwrap();
        let r=parse_request_file(&p).unwrap();
        assert_eq!(r.name,"Get Profile");
        assert_eq!(r.url,"https://api.example.com/profile");
        assert_eq!(r.headers.len(),1);
    }
    #[test]
    fn opencollection_collection_name_and_skip() {
        let d=std::env::temp_dir().join("lb-opencollection-fixture");
        let _=fs::remove_dir_all(&d); fs::create_dir_all(&d).unwrap();
        fs::write(d.join("opencollection.yml"),"opencollection: 1.0.0\n\ninfo://n  name: gtgj\nbundled: false\n").unwrap();
        fs::write(d.join("query.yml"),"info://n  name: queryUserInfo\n  type: http\n  seq: 12\nhttp://n  method: GET\n  url: \"{{ip}}/x\"\n  auth: inherit\n").unwrap();
        assert_eq!(collection_name(&d), "gtgj");
        assert!(is_collection_dir(&d));
        let mut ws=Workspace::default();
        let root=d.parent().unwrap().to_path_buf();
        scan_root(&root, &mut ws);
        // opencollection.yml 必须被跳过，只留 1 条请求
        let c=ws.collections.iter().find(|c| c.name=="gtgj").expect("collection gtgj");
        assert_eq!(c.requests.len(),1);
        assert_eq!(c.requests[0].name,"queryUserInfo");
    }

    #[test]
    fn safe_name_sanitizes() {
        assert_eq!(safe_name("my:col/lection"),"my_col_lection");
        assert_eq!(safe_name(".."),"collection");
    }

    #[test]
    fn probe_external_bruno_repo() {
        // 单个集合目录
        let r = probe_bruno_repo(r"C://work//ext-bruno-repo//MyAPI");
        assert!(r.is_ok(), "{:?}", r);
        let (name, cnt) = r.unwrap();
        assert_eq!(name, "External Bruno Repo");
        assert_eq!(cnt, 2); // list.yml + create.bru
    }
    #[test]
    fn scan_external_collection_reads_both_formats() {
        let root = std::path::Path::new(r"C://work//ext-bruno-repo");
        let mut ws = Workspace::default();
        scan_root(root, &mut ws);
        assert_eq!(ws.collections.len(), 1);
        let c = &ws.collections[0];
        assert_eq!(c.name, "External Bruno Repo");
        assert_eq!(c.requests.len(), 2);
        assert!(c.requests.iter().any(|r| r.name == "Get Users"));
        assert!(c.requests.iter().any(|r| r.name == "Create Post"));
    }
    #[test]
    fn scan_merges_environment() {
        let root = std::path::Path::new(r"C://work//ext-bruno-repo");
        let mut ws = Workspace::default();
        scan_root(root, &mut ws);
        assert!(ws.environments.contains_key("local"));
        assert_eq!(ws.environments.get("local").unwrap().get("baseUrl").unwrap(),
                   "https://jsonplaceholder.typicode.com");
    }
}
