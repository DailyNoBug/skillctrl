export type JsonRecord = Record<string, any>;

export interface CommandExecution {
  success: boolean;
  stdout: string;
  stderr: string;
  json: JsonRecord | null;
  command_line: string;
  binary_path: string;
}

export interface SourceRecord {
  name: string;
  repo_url: string;
  branch: string;
  auth: string;
  last_commit?: string | null;
  updated_at?: string | null;
}

export interface AssetRecord {
  id: string;
  name: string;
  source: string;
  version: string;
  asset_types: string[];
  targets: string[];
  summary: string;
}

export interface BundleComponent {
  id: string;
  kind: string;
  path: string;
  description?: string | null;
}

export interface BundleDetail {
  id: string;
  name: string;
  source: string;
  version: string;
  targets: string[];
  description?: string | null;
  base_path: string;
  components: BundleComponent[];
}

export interface InstallRecord {
  bundle_id: string;
  version: string;
  source_name?: string | null;
  endpoint: string;
  scope: string;
  project_path?: string | null;
  installed_at: string;
  files_created: string[];
}

export interface VerificationFile {
  path: string;
  exists: boolean;
  matches_expected: boolean;
  detail: string;
}

export interface VerificationComponent {
  id: string;
  kind: string;
  installed: boolean;
  content_matches: boolean;
  detail: string;
  files: VerificationFile[];
}

export interface VerificationResult {
  bundle_id: string;
  source: string;
  target: string;
  scope: string;
  project_path?: string | null;
  installed: boolean;
  installed_version?: string | null;
  latest_version: string;
  is_latest_version: boolean;
  local_matches_source: boolean;
  installation_record_found: boolean;
  files_checked: number;
  files_matching: number;
  components: VerificationComponent[];
}

export interface ImportArtifactRecord {
  kind: string;
  id?: string | null;
  path: string;
  description?: string | null;
  supported?: boolean | null;
}
