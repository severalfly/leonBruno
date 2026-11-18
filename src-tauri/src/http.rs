use crate::types::{ApiRequest, ApiResponse, KeyValue};
use std::collections::BTreeMap;
use std::time::Instant;

fn interpolate(s: &str, vars: &BTreeMap<String, String>) -> String {
    let mut out = s.to_string();
    // 简单 {{var}} 替换，最多 5 轮以支持变量引用变量
    for _ in 0..5 {
        let mut changed = false;
        for (k, v) in vars {
            let pat = format!("{{{{{}}}}}", k);
            if out.contains(&pat) { out = out.replace(&pat, v); changed = true; }
        }
        if !changed { break; }
    }
    out
}

/// reqwest 的 {} 只给最外层，展开 source 链才能看到 DNS/TLS/连接/超时的真正原因。
fn err_chain(e: &reqwest::Error) -> String {
    let mut out = format!("{}", e);
    let mut cur: Option<&(dyn std::error::Error + 'static)> = std::error::Error::source(e);
    let mut n = 0;
    while let Some(src) = cur {
        n += 1;
        out.push_str(&format!("\n  [{}] {}", n, src));
        cur = src.source();
        if n >= 8 { break; }
    }
    out
}

pub async fn execute(req: ApiRequest, env: BTreeMap<String, String>, timeout_secs: u64) -> Result<ApiResponse, String> {
    let method = reqwest::Method::from_bytes(req.method.as_str().as_bytes()).map_err(|e| e.to_string())?;
    let mut url = interpolate(&req.url, &env);
    // 没有 scheme 时补 https（现代站点普遍 https，http 会多一次 301 跳转且
    // 在跨协议重定向时 reqwest 会丢掉 Authorization header）
    if !url.starts_with("http://") && !url.starts_with("https://") { url = format!("https://{}", url); }
    if !req.params.is_empty() {
        let have_q = url.contains('?');
        let mut parts = Vec::new();
        for p in req.params.iter().filter(|p| p.enabled && !p.key.trim().is_empty()) {
            let k = interpolate(&p.key, &env);
            let v = interpolate(&p.value, &env);
            parts.push(format!("{}={}", urlencoding(&k), urlencoding(&v)));
        }
        if !parts.is_empty() {
            url.push(if have_q { '&' } else { '?' });
            url.push_str(&parts.join("&"));
        }
    }

    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .redirect(reqwest::redirect::Policy::limited(5));
    // 忽略证书错误（内部测试环境常见自签证书）
    #[allow(deprecated)]
    { builder = builder.danger_accept_invalid_certs(true); }
    let client = builder.build().map_err(|e| err_chain(&e))?;

    let mut rb = client.request(method, &url);
    for h in req.headers.iter().filter(|h| h.enabled && !h.key.trim().is_empty()) {
        let k = interpolate(&h.key, &env);
        let v = interpolate(&h.value, &env);
        rb = rb.header(&k, &v);
    }
    match req.auth.kind.as_str() {
        "bearer" if !req.auth.token.is_empty() => {
            rb = rb.bearer_auth(interpolate(&req.auth.token, &env));
        }
        "basic" => {
            rb = rb.basic_auth(interpolate(&req.auth.username, &env), Some(interpolate(&req.auth.password, &env)));
        }
        _ => {}
    }
    let body = interpolate(&req.body, &env);
    if !body.is_empty() && req.body_type != "none" {
        rb = rb.body(body);
    }

    let started = Instant::now();
    let resp = rb.send().await.map_err(|e| err_chain(&e))?;
    let status = resp.status().as_u16();
    let status_text = resp.status().canonical_reason().unwrap_or("").to_string();
    let headers: Vec<KeyValue> = resp.headers().iter().map(|(k, v)| KeyValue {
        key: k.to_string(), value: v.to_str().unwrap_or("<binary>").to_string(), enabled: true,
    }).collect();
    let bytes = resp.bytes().await.map_err(|e| err_chain(&e))?;
    let elapsed = started.elapsed().as_millis();
    let size = bytes.len();
    let text = String::from_utf8_lossy(&bytes).to_string();
    Ok(ApiResponse { status, status_text, headers, body: text, elapsed_ms: elapsed, size })
}

fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}


