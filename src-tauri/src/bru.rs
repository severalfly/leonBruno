use crate::types::{ApiRequest, Auth, HttpMethod, KeyValue};

/// Bru Markup Language 块：tag { ... }
#[derive(Debug)]
struct Block { tag: String, lines: Vec<String> }

/// 把 .bru 文本切成若干块。块外文本忽略，@description(...) 行忽略。
fn parse_blocks(text: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut cur: Option<Block> = None;
    for raw in text.lines() {
        let line = raw.trim_end();
        let t = line.trim();
        if let Some(b) = cur.as_mut() {
            if t == "}" {
                out.push(b.take());
                cur = None;
            } else if !t.starts_with("@description") {
                b.lines.push(line.to_string());
            }
        } else if t.ends_with('{') && !t.starts_with('@') {
            let tag = t[..t.len()-1].trim().to_string();
            cur = Some(Block { tag, lines: Vec::new() });
        }
    }
    if let Some(b) = cur { out.push(b); }
    out
}
impl Block {
    fn take(&mut self) -> Block { std::mem::replace(self, Block { tag: String::new(), lines: Vec::new() }) }
}

/// 解析一行 `key: value`，`~key` 前缀表示 disabled。
fn kv_line(line: &str) -> Option<(String, String, bool)> {
    let t = line.trim();
    if t.is_empty() || t.starts_with('@') { return None; }
    let (disabled, rest) = if let Some(r) = t.strip_prefix('~') { (true, r) } else { (false, t) };
    let p = rest.find(':')?;
    Some((rest[..p].trim().to_string(), rest[p+1..].trim().to_string(), disabled))
}

/// 解析 Bruno 官方 .bru 文件为 ApiRequest。
pub fn parse(text: &str, file_id: &str) -> Result<ApiRequest, String> {
    let blocks = parse_blocks(text);
    let mut req = ApiRequest::default();
    req.id = file_id.to_string();

    let mut body_type = String::new();
    let mut body_lines: Vec<String> = Vec::new();
    let mut auth_kind = String::new();
    let mut bearer_token = String::new();
    let mut basic_user = String::new();
    let mut basic_pass = String::new();
    let mut apikey_key = String::new();
    let mut apikey_value = String::new();
    let mut apikey_placement = "header".to_string();

    for b in &blocks {
        match b.tag.as_str() {
            "meta" => {
                for l in &b.lines {
                    if let Some((k,v,_)) = kv_line(l) {
                        match k.as_str() {
                            "name" => req.name = v,
                            "seq" => req.seq = v.parse().unwrap_or(0),
                            _ => {}
                        }
                    }
                }
            }
            "get" | "post" | "put" | "patch" | "delete" | "head" | "options" | "trace" | "connect" => {
                req.method = HttpMethod::parse(&b.tag);
                for l in &b.lines {
                    if let Some((k,v,_)) = kv_line(l) {
                        match k.as_str() {
                            "url" => req.url = v.trim_matches(|c| c=='\'' || c=='"').to_string(),
                            "body" => body_type = v.to_lowercase(),
                            "auth" => auth_kind = v.to_lowercase(),
                            _ => {}
                        }
                    }
                }
            }
            "params:query" | "query" => {
                for l in &b.lines {
                    if let Some((k,v,off)) = kv_line(l) {
                        if k.starts_with('~') { continue; }
                        req.params.push(KeyValue { key:k, value:v.trim_matches(|c| c=='\''||c=='"').to_string(), enabled:!off });
                    }
                }
            }
            "headers" => {
                for l in &b.lines {
                    if let Some((k,v,off)) = kv_line(l) {
                        req.headers.push(KeyValue { key:k, value:v.trim_matches(|c| c=='\''||c=='"').to_string(), enabled:!off });
                    }
                }
            }
            "body" | "body:json" | "body:text" | "body:xml" | "body:form-urlencoded" | "body:multipart-form" | "body:graphql" | "body:graphql:vars" => {
                body_type = match b.tag.as_str() {
                    "body:text" => "text", "body:xml" => "xml",
                    "body:form-urlencoded" => "form", "body:multipart-form" => "form",
                    "body:graphql" | "body:graphql:vars" => "graphql",
                    _ => "json",
                }.to_string();
                body_lines = b.lines.clone();
            }
            "auth:bearer" => {
                auth_kind = "bearer".into();
                for l in &b.lines {
                    if let Some((k,v,_)) = kv_line(l) { if k=="token" { bearer_token=v; } }
                }
            }
            "auth:basic" => {
                auth_kind = "basic".into();
                for l in &b.lines {
                    if let Some((k,v,_)) = kv_line(l) {
                        match k.as_str() { "username"=>basic_user=v, "password"=>basic_pass=v, _=>{} }
                    }
                }
            }
            "auth:apikey" => {
                auth_kind = "apikey".into();
                for l in &b.lines {
                    if let Some((k,v,_)) = kv_line(l) {
                        match k.as_str() { "key"=>apikey_key=v, "value"=>apikey_value=v, "placement"=>apikey_placement=v, _=>{} }
                    }
                }
            }
            "tests" | "script:pre-request" | "script:post-response" | "docs" | "vars" | "vars:secret" => { /* 保留原文件，UI 暂不展示 */ }
            _ => {}
        }
    }

    if !body_lines.is_empty() {
        if body_type == "form" {
            let mut parts = Vec::new();
            for l in &body_lines {
                if let Some((k,v,off)) = kv_line(l) {
                    if k.starts_with('~') { continue; }
                    parts.push(format!("{}={}", urlenc(&k), urlenc(v.trim_matches(|c| c=='\''||c=='"'))));
                }
            }
            req.body = parts.join("&");
        } else {
            req.body = body_lines.join("\n").trim().to_string();
        }
    }
    req.body_type = if req.body.is_empty() { "none".into() } else { body_type };

    req.auth = match auth_kind.as_str() {
        "bearer" => Auth { kind:"bearer".into(), token:bearer_token, ..Default::default() },
        "basic" => Auth { kind:"basic".into(), username:basic_user, password:basic_pass, ..Default::default() },
        "apikey" => {
            if apikey_placement == "header" {
                req.headers.push(KeyValue { key:apikey_key, value:apikey_value, enabled:true });
            }
            Auth { kind:"none".into(), ..Default::default() }
        }
        _ => Auth { kind:"none".into(), ..Default::default() },
    };

    if req.name.is_empty() { req.name = file_id.to_string(); }
    Ok(req)
}

fn urlenc(s:&str)->String {
    let mut o=String::new();
    for b in s.bytes() {
        match b { b'A'..=b'Z'|b'a'..=b'z'|b'0'..=b'9'|b'-'|b'_'|b'.'|b'~'=>o.push(b as char), _=>o.push_str(&format!("%{:02X}",b)) }
    }
    o
}

/// 把 ApiRequest 序列化成 Bruno 可直接打开的 .bru 文件。
pub fn stringify(req: &ApiRequest) -> String {
    let mut s = String::new();
    s.push_str("meta {\n");
    s.push_str(&format!("  name: {}\n", req.name));
    s.push_str("  type: http\n");
    s.push_str(&format!("  seq: {}\n", if req.seq==0 {1} else {req.seq}));
    s.push_str("}\n\n");

    s.push_str(&format!("{} {{\n", req.method.as_str().to_lowercase()));
    s.push_str(&format!("  url: {}\n", req.url));
    let bt = if req.body_type=="none" || req.body.is_empty() { "none".to_string() }
        else if req.body_type=="form" { "form-urlencoded".to_string() }
        else { req.body_type.clone() };
    s.push_str(&format!("  body: {}\n", bt));
    s.push_str(&format!("  auth: {}\n", if req.auth.kind=="none" {"none"} else {&req.auth.kind}));
    s.push_str("}\n\n");

    let active_params: Vec<_> = req.params.iter().filter(|p| !p.key.trim().is_empty()).collect();
    if !active_params.is_empty() {
        s.push_str("params:query {\n");
        for p in &active_params {
            s.push_str(&format!("  {}{}: {}\n", if p.enabled {""} else {"~"}, p.key, p.value));
        }
        s.push_str("}\n\n");
    }
    let active_headers: Vec<_> = req.headers.iter().filter(|h| !h.key.trim().is_empty()).collect();
    if !active_headers.is_empty() {
        s.push_str("headers {\n");
        for h in &active_headers {
            s.push_str(&format!("  {}{}: {}\n", if h.enabled {""} else {"~"}, h.key, h.value));
        }
        s.push_str("}\n\n");
    }
    match req.auth.kind.as_str() {
        "bearer" => { s.push_str("auth:bearer {\n"); s.push_str(&format!("  token: {}\n",req.auth.token)); s.push_str("}\n\n"); }
        "basic" => { s.push_str("auth:basic {\n"); s.push_str(&format!("  username: {}\n  password: {}\n",req.auth.username,req.auth.password)); s.push_str("}\n\n"); }
        _ => {}
    }
    if !req.body.is_empty() && req.body_type != "none" {
        match req.body_type.as_str() {
            "text" => s.push_str(&format!("body:text {{\n{}\n}}\n", req.body)),
            "xml" => s.push_str(&format!("body:xml {{\n{}\n}}\n", req.body)),
            "form" => {
                s.push_str("body:form-urlencoded {\n");
                for part in req.body.split('&') {
                    if part.is_empty() { continue; }
                    let mut it=part.splitn(2,'=');
                    s.push_str(&format!("  {}: {}\n", urldec(it.next().unwrap_or("")), urldec(it.next().unwrap_or(""))));
                }
                s.push_str("}\n");
            }
            _ => s.push_str(&format!("body:json {{\n{}\n}}\n", req.body)),
        }
    }
    s
}

fn urldec(s:&str)->String {
    let b=s.as_bytes(); let mut o=Vec::new(); let mut i=0;
    while i<b.len() {
        if b[i]==b'%' && i+2<b.len() {
            let h=|x:u8|->Option<u8>{match x{b'0'..=b'9'=>Some(x-b'0'),b'a'..=b'f'=>Some(x-b'a'+10),b'A'..=b'F'=>Some(x-b'A'+10),_=>None}};
            if let (Some(a),Some(c))=(h(b[i+1]),h(b[i+2])) { o.push(a*16+c); i+=3; continue; }
        }
        o.push(b[i]); i+=1;
    }
    String::from_utf8_lossy(&o).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_bru_official_sample() {
        let txt = r#"meta {
  name: Get users
  type: http
  seq: 1
}

get {
  url: https://api.github.com/users/usebruno
}

headers {
  content-type: application/json
  Authorization: Bearer topsecret
}
"#;
        let r = parse(txt, "get-users").unwrap();
        assert_eq!(r.name,"Get users");
        assert_eq!(r.method,HttpMethod::Get);
        assert_eq!(r.url,"https://api.github.com/users/usebruno");
        assert_eq!(r.headers.len(),2);
    }
    #[test]
    fn parse_post_with_body_auth_params_disabled() {
        let txt = r#"meta { name: Create User type: http seq: 2 }
post {
  url: https://api.example.com/users
  body: json
  auth: bearer
}
params:query {
  page: 2
  ~debug: true
}
headers {
  Content-Type: application/json
}
auth:bearer {
  token: abc123
}
body:json {
  {"name":"leon"}
}
"#;
        let r = parse(txt,"create-user").unwrap();
        assert_eq!(r.method,HttpMethod::Post);
        assert_eq!(r.params.len(),2);
        assert_eq!(r.params[0].key,"page");
        assert!(!r.params[1].enabled);
        assert_eq!(r.auth.kind,"bearer");
        assert_eq!(r.auth.token,"abc123");
        assert!(r.body.contains("leon"));
        assert_eq!(r.body_type,"json");
    }
    #[test]
    fn roundtrip() {
        let mut r=ApiRequest::default();
        r.id="r1".into(); r.name="Test".into(); r.method=HttpMethod::Post;
        r.url="https://x.test/api".into(); r.body_type="json".into(); r.body="{\"a\":1}".into();
        r.params.push(KeyValue{key:"q".into(),value:"1".into(),enabled:true});
        r.headers.push(KeyValue{key:"X-T".into(),value:"v".into(),enabled:true});
        r.auth=Auth{kind:"bearer".into(),token:"t".into(),..Default::default()};
        let txt=stringify(&r);
        let back=parse(&txt,"r1").unwrap();
        assert_eq!(back.name,"Test");
        assert_eq!(back.method,HttpMethod::Post);
        assert_eq!(back.url,"https://x.test/api");
        assert_eq!(back.params.len(),1);
        assert_eq!(back.headers.len(),1);
        assert_eq!(back.auth.token,"t");
        assert_eq!(back.body,"{\"a\":1}");
    }
}
