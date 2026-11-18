export type HttpMethod = 'get' | 'post' | 'put' | 'patch' | 'delete' | 'head' | 'options'
export interface KeyValue { key: string; value: string; enabled: boolean }
export interface Auth {
  kind: string
  token: string
  username: string
  password: string
}
export interface ApiRequest {
  id: string
  name: string
  seq?: number
  method: HttpMethod
  url: string
  headers: KeyValue[]
  params: KeyValue[]
  body: string
  body_type: string
  auth: Auth
}
export interface ApiResponse {
  status: number
  status_text: string
  headers: KeyValue[]
  body: string
  elapsed_ms: number
  size: number
}
export interface Collection { id: string; name: string; requests: ApiRequest[] }
export interface Workspace {
  collections: Collection[]
  environments: Record<string, Record<string, string>>
}
export interface AppSettings {
  workspace_dir: string
  active_environment: string
  timeout_secs: number
  format?: string
}
export function blankRequest(): ApiRequest {
  return {
    id: `r${Date.now()}`,
    name: 'New Request',
    seq: 0,
    method: 'get',
    url: '',
    headers: [{ key: '', value: '', enabled: true }],
    params: [{ key: '', value: '', enabled: true }],
    body: '',
    body_type: 'none',
    auth: { kind: 'none', token: '', username: '', password: '' },
  }
}
