use crate::types::{ApiRequest, Auth, HttpMethod, KeyValue};

/// 解析从浏览器 DevTools / 终端复制的 curl 命令。
/// 支持 -X/-H/-d/--data/--data-raw/-b/-u/-A/-k/-L/--compressed/-F 以及 URL 上的 query。
pub fn parse_curl(input: &str) -> Result<ApiRequest, String> {
    let text = input.trim();
    if !text.to_lowercase().starts_with("curl") {
        return Err("不是 curl 命令".into());
    }
    // 把反斜杠+换行合并成空格；支持反引号/引号包裹的 URL
    let flat = text.replace("\\\r\n", " ").replace("\\\n", " ").replace("\r\n", " ").replace("\n", " ");
    let tokens = shell_split(&flat)?;
    let mut req = ApiRequest::default();
    let mut i = 1usize;
    let mut data_parts: Vec<String> = Vec::new();
    let mut form_parts: Vec<String> = Vec::new();
    let mut basic: Option<String> = None;
    let mut cookie: Option<String> = None;

    while i < tokens.len() {
        let t = &tokens[i];
        match t.as_str() {
            "-X" | "--request" => { i += 1; if i < tokens.len() { req.method = HttpMethod::parse(&tokens[i]); } }
            "-H" | "--header" => {
                i += 1;
                if i < tokens.len() {
                    if let Some((k, v)) = split_header(&tokens[i]) {
                        if k.eq_ignore_ascii_case("authorization") {
                            if let Some(tok) = v.strip_prefix("Bearer ").or_else(|| v.strip_prefix("bearer ")) {
                                req.auth = Auth { kind: "bearer".into(), token: tok.trim().into(), ..Default::default() };
                            } else {
                                req.headers.push(kv(k, v));
                            }
                        } else if k.eq_ignore_ascii_case("cookie") {
                            cookie = Some(v.to_string());
                        } else {
                            req.headers.push(kv(k, v));
                        }
                    }
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-ascii" | "--data-binary" => {
                i += 1; if i < tokens.len() { data_parts.push(tokens[i].clone()); }
            }
            "-F" | "--form" => { i += 1; if i < tokens.len() { form_parts.push(tokens[i].clone()); } }
            "-b" | "--cookie" => { i += 1; if i < tokens.len() { cookie = Some(tokens[i].clone()); } }
            "-u" | "--user" => { i += 1; if i < tokens.len() { basic = Some(tokens[i].clone()); } }
            "-A" | "--user-agent" => {
                i += 1; if i < tokens.len() { req.headers.push(kv("User-Agent", &tokens[i])); }
            }
            "-k" | "--insecure" | "-L" | "--location" | "--compressed" | "-s" | "--silent" | "-v" | "--verbose" | "--http1.1" | "--http2" => {}
            "--url" => { i += 1; if i < tokens.len() { req.url = tokens[i].trim_matches(|c| c == '\'' || c == '"').to_string(); } }
            _ => {
                if !t.starts_with('-') && req.url.is_empty() {
                    req.url = t.trim_matches(|c| c == '\'' || c == '"').to_string();
                }
            }
        }
        i += 1;
    }

    if req.url.is_empty() { return Err("curl 里没有找到 URL".into()); }

    // URL query 拆到参数表
    if let Some(qpos) = req.url.find('?') {
        let query = req.url[qpos + 1..].to_string();
        req.url = req.url[..qpos].to_string();
        for pair in query.split('&') {
            if pair.is_empty() { continue; }
            let mut it = pair.splitn(2, '=');
            let k = percent_decode(it.next().unwrap_or(""));
            let v = percent_decode(it.next().unwrap_or(""));
            req.params.push(KeyValue { key: k, value: v, enabled: true });
        }
    }

    if !data_parts.is_empty() {
        req.body = data_parts.join("&");
        req.body_type = "raw".into();
        if req.method == HttpMethod::Get { req.method = HttpMethod::Post; }
        if !req.headers.iter().any(|h| h.key.eq_ignore_ascii_case("content-type")) {
            req.headers.push(kv("Content-Type", "application/x-www-form-urlencoded"));
        }
    } else if !form_parts.is_empty() {
        req.body = form_parts.join("\n");
        req.body_type = "form".into();
        if req.method == HttpMethod::Get { req.method = HttpMethod::Post; }
    }

    if let Some(c) = cookie { req.headers.push(kv("Cookie", &c)); }
    if let Some(b) = basic {
        let (u, p) = b.split_once(':').unwrap_or((&b, ""));
        req.auth = Auth { kind: "basic".into(), username: u.into(), password: p.into(), ..Default::default() };
    }

    req.name = {
        let tail = req.url.rsplit('/').next().unwrap_or("request");
        let tail = tail.split('?').next().unwrap_or("request");
        if tail.is_empty() { "request".into() } else { tail[..tail.len().min(40)].to_string() }
    };
    Ok(req)
}

fn kv(k: &str, v: &str) -> KeyValue { KeyValue { key: k.trim().to_string(), value: v.trim().to_string(), enabled: true } }

fn split_header(s: &str) -> Option<(&str, &str)> {
    let p = s.find(':')?;
    Some((&s[..p], s[p + 1..].trim()))
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(a), Some(b)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push(a * 16 + b); i += 3; continue;
            }
        }
        if bytes[i] == b'+' { out.push(b' '); } else { out.push(bytes[i]); }
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}
fn hex(b: u8) -> Option<u8> {
    match b { b'0'..=b'9' => Some(b - b'0'), b'a'..=b'f' => Some(b - b'a' + 10), b'A'..=b'F' => Some(b - b'A' + 10), _ => None }
}

/// 简单 shell 分词：支持单引号、双引号与反斜杠转义。
fn shell_split(s: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match quote {
            Some('\'') => { if c == '\'' { quote = None; } else { cur.push(c); } }
            Some('"') => {
                if c == '"' { quote = None; }
                else if c == '\\' {
                    if let Some(&n) = chars.peek() { cur.push(n); chars.next(); }
                } else { cur.push(c); }
            }
            _ => match c {
                '\'' | '"' => quote = Some(c),
                '\\' => { if let Some(&n) = chars.peek() { cur.push(n); chars.next(); } }
                c if c.is_whitespace() => {
                    if !cur.is_empty() { out.push(std::mem::take(&mut cur)); }
                }
                _ => cur.push(c),
            },
        }
    }
    if quote.is_some() { return Err("引号未闭合".into()); }
    if !cur.is_empty() { out.push(cur); }
    Ok(out)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_get() {
        let r = parse_curl("curl https://api.example.com/users").unwrap();
        assert_eq!(r.method, HttpMethod::Get);
        assert_eq!(r.url, "https://api.example.com/users");
    }

    #[test]
    fn parse_devtools_post_with_headers_body() {
        let cmd = "curl 'https://api.example.com/v1/users?page=2&q=hello' \\\n  -X POST \\\n  -H 'Authorization: Bearer abc123' \\\n  -H 'Content-Type: application/json' \\\n  -H 'X-Trace: t-99' \\\n  --data-raw '{\"name\":\"leon\",\"age\":18}'";
        let r = parse_curl(cmd).unwrap();
        assert_eq!(r.method, HttpMethod::Post);
        assert_eq!(r.url, "https://api.example.com/v1/users");
        assert_eq!(r.params.len(), 2);
        assert_eq!(r.params[0].key, "page");
        assert_eq!(r.params[0].value, "2");
        assert_eq!(r.params[1].key, "q");
        assert_eq!(r.params[1].value, "hello");
        assert_eq!(r.auth.kind, "bearer");
        assert_eq!(r.auth.token, "abc123");
        assert!(r.headers.iter().any(|h| h.key == "X-Trace" && h.value == "t-99"));
        assert!(r.body.contains("leon"));
    }

    #[test]
    fn parse_cookie_and_basic() {
        let cmd = "curl -u alice:secret -b 'session=xyz; theme=dark' 'http://localhost:8080/api'";
        let r = parse_curl(cmd).unwrap();
        assert_eq!(r.auth.kind, "basic");
        assert_eq!(r.auth.username, "alice");
        assert_eq!(r.auth.password, "secret");
        assert!(r.headers.iter().any(|h| h.key == "Cookie" && h.value.contains("session=xyz")));
    }

    #[test]
    fn parse_data_without_method_defaults_post() {
        let r = parse_curl("curl -d 'a=1&b=2' https://httpbin.org/post").unwrap();
        assert_eq!(r.method, HttpMethod::Post);
        assert_eq!(r.body, "a=1&b=2");
        assert_eq!(r.body_type, "raw");
    }

    #[test]
    fn parse_encoded_query() {
        let r = parse_curl("curl 'https://api.test/search?q=hello%20world&tag=a+b'").unwrap();
        assert_eq!(r.params[0].value, "hello world");
        assert_eq!(r.params[1].value, "a b");
    }
}
