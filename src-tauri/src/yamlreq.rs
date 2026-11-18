use crate::types::{ApiRequest, Auth, HttpMethod, KeyValue};

/// OpenCollection YAML（Bruno 新推荐格式）读取。
/// 只解析我们关心的字段，脚本/断言保留在原文件里，不丢。
#[derive(serde::Deserialize, Default)]
struct YamlReq {
    #[serde(default)] info: Info,
    #[serde(default)] http: Http,
}
#[derive(serde::Deserialize, Default)]
struct Info { #[serde(default)] name: String, #[serde(default)] seq: u32 }
#[derive(serde::Deserialize, Default)]
struct Http {
    #[serde(default)] method: String,
    #[serde(default)] url: String,
    #[serde(default)] params: Vec<Ykv>,
    #[serde(default)] headers: Vec<Ykv>,
    #[serde(default)] body: Option<YBody>,
    #[serde(default)] auth: Option<YAuth>,
}
#[derive(serde::Deserialize, Default, Clone)]
struct Ykv {
    #[serde(default)] name: String,
    #[serde(default)] value: String,
    #[serde(default)] r#type: String,
    #[serde(default)] disabled: bool,
}
#[derive(serde::Deserialize, Default)]
struct YBody { #[serde(default)] r#type: String, #[serde(default)] data: serde_json::Value }
#[derive(serde::Deserialize, Default)]
struct YAuth {
    #[serde(default)] r#type: String,
    #[serde(default)] token: String,
    #[serde(default)] username: String,
    #[serde(default)] password: String,
    #[serde(default)] key: String,
    #[serde(default)] value: String,
}

pub fn parse(text: &str, file_id: &str) -> Result<ApiRequest, String> {
    let y: YamlReq = serde_yaml::from_str(text).map_err(|e| e.to_string())?;
    let mut r = ApiRequest::default();
    r.id = file_id.to_string();
    r.name = if y.info.name.is_empty() { file_id.to_string() } else { y.info.name };
    r.seq = y.info.seq;
    r.method = HttpMethod::parse(&y.http.method);
    r.url = y.http.url;
    for p in &y.http.params {
        if p.r#type == "path" { continue; }
        r.params.push(KeyValue{key:p.name.clone(),value:p.value.clone(),enabled:!p.disabled});
    }
    for h in &y.http.headers {
        r.headers.push(KeyValue{key:h.name.clone(),value:h.value.clone(),enabled:!h.disabled});
    }
    if let Some(b) = &y.http.body {
        r.body_type = match b.r#type.as_str() {
            "form-urlencoded"|"multipart-form" => "form".into(),
            "text"|"xml"|"graphql" => b.r#type.clone(),
            _ => "json".into(),
        };
        r.body = match &b.data {
            serde_json::Value::String(s) => {
                if r.body_type=="form" {
                    s.lines().filter_map(|l| {
                        let mut it=l.splitn(2,':');
                        let k=it.next()?.trim(); let v=it.next().unwrap_or("").trim();
                        if k.is_empty(){None}else{Some(format!("{}={}",k,v))}
                    }).collect::<Vec<_>>().join("&")
                } else { s.clone() }
            }
            other => serde_json::to_string_pretty(other).unwrap_or_default(),
        };
    }
    if let Some(a) = &y.http.auth {
        r.auth = match a.r#type.as_str() {
            "bearer" => Auth{kind:"bearer".into(),token:a.token.clone(),..Default::default()},
            "basic" => Auth{kind:"basic".into(),username:a.username.clone(),password:a.password.clone(),..Default::default()},
            "apikey" => { r.headers.push(KeyValue{key:a.key.clone(),value:a.value.clone(),enabled:true}); Auth::default() }
            _ => Auth::default(),
        };
    }
    Ok(r)
}

/// 序列化成 OpenCollection YAML（用于「格式 = YAML」时保存）
pub fn stringify(r: &ApiRequest) -> String {
    let mut s=String::new();
    s.push_str("info:/n");
    s.push_str(&format!("  name: {}\n  type: http\n  seq: {}\n", yaml_str(&r.name), if r.seq==0 {1} else {r.seq}));
    s.push_str("http:/n");
    s.push_str(&format!("  method: {}\n  url: {}\n", r.method.as_str(), yaml_str(&r.url)));
    let params:Vec<_>=r.params.iter().filter(|p|!p.key.trim().is_empty()).collect();
    if !params.is_empty() {
        s.push_str("  params:/n");
        for p in params { s.push_str(&format!("    - name: {}\n      value: {}\n      type: query\n", yaml_str(&p.key), yaml_str(&p.value))); }
    }
    let headers:Vec<_>=r.headers.iter().filter(|h|!h.key.trim().is_empty()).collect();
    if !headers.is_empty() {
        s.push_str("  headers:/n");
        for h in headers { s.push_str(&format!("    - name: {}\n      value: {}\n", yaml_str(&h.key), yaml_str(&h.value))); }
    }
    if !r.body.is_empty() && r.body_type!="none" {
        let t=if r.body_type=="form" {"form-urlencoded"} else {&r.body_type};
        s.push_str(&format!("  body:/n    type: {}\n    data: |-\n",t));
        for l in r.body.lines() { s.push_str(&format!("      {}\n",l)); }
    }
    if r.auth.kind=="bearer" {
        s.push_str(&format!("  auth:/n    type: bearer\n    token: {}\n",yaml_str(&r.auth.token)));
    } else if r.auth.kind=="basic" {
        s.push_str(&format!("  auth:/n    type: basic\n    username: {}\n    password: {}\n",yaml_str(&r.auth.username),yaml_str(&r.auth.password)));
    }
    s.push_str("settings:/n  encodeUrl: true\n");
    s
}
fn yaml_str(s:&str)->String {
    if s.is_empty() { "\"\"".into() }
    else if s.contains(':')||s.contains('#')||s.starts_with('{')||s.starts_with('[') { format!("\"{}\"",s.replace('\\',"\\\\").replace('"',"\\\"")) }
    else { s.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_yml_sample() {
        let t=r#"info:
  name: Get User
  type: http
  seq: 1
http:
  method: GET
  url: https://api.github.com/users/usebruno
settings:
  encodeUrl: true
"#;
        let r=parse(t,"get-user").unwrap();
        assert_eq!(r.name,"Get User");
        assert_eq!(r.method,HttpMethod::Get);
        assert_eq!(r.url,"https://api.github.com/users/usebruno");
    }
}
