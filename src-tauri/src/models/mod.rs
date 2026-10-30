//! Data model shared between scanner, resolver, storage and the frontend.
//! Everything serializes as camelCase.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Scope {
    Managed,
    User,
    Project,
    ProjectLocal,
    Plugin,
    Nested,
    GlobalConfig,
}

impl Scope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Scope::Managed => "managed",
            Scope::User => "user",
            Scope::Project => "project",
            Scope::ProjectLocal => "projectLocal",
            Scope::Plugin => "plugin",
            Scope::Nested => "nested",
            Scope::GlobalConfig => "globalConfig",
        }
    }

    /// Settings precedence rank: lower wins. `None` if the scope does not
    /// participate in settings resolution.
    pub fn settings_rank(&self) -> Option<u8> {
        match self {
            Scope::Managed => Some(0),
            Scope::ProjectLocal => Some(1),
            Scope::Project => Some(2),
            Scope::User => Some(3),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactKind {
    Settings,
    ClaudeMd,
    Rule,
    Skill,
    Command,
    Agent,
    Hook,
    McpServer,
    Plugin,
    Memory,
    OutputStyle,
    Workflow,
    Other,
}

impl ArtifactKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ArtifactKind::Settings => "settings",
            ArtifactKind::ClaudeMd => "claudeMd",
            ArtifactKind::Rule => "rule",
            ArtifactKind::Skill => "skill",
            ArtifactKind::Command => "command",
            ArtifactKind::Agent => "agent",
            ArtifactKind::Hook => "hook",
            ArtifactKind::McpServer => "mcpServer",
            ArtifactKind::Plugin => "plugin",
            ArtifactKind::Memory => "memory",
            ArtifactKind::OutputStyle => "outputStyle",
            ArtifactKind::Workflow => "workflow",
            ArtifactKind::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiscoveryStatus {
    Present,
    Missing,
    Unreadable,
    InvalidJson,
    InvalidFrontmatter,
    Partial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Confidence {
    Computed,
    Inferred,
    Unknown,
}

fn short_hash(input: &str) -> String {
    let mut h = Sha256::new();
    h.update(input.as_bytes());
    hex::encode(h.finalize())[..16].to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigArtifact {
    pub id: String,
    pub kind: ArtifactKind,
    pub scope: Scope,
    pub name: String,
    pub path: String,
    pub project_path: Option<String>,
    pub status: DiscoveryStatus,
    pub size_bytes: Option<u64>,
    pub modified_at: Option<String>,
    pub description: Option<String>,
    pub error: Option<String>,
    /// File tree listing (relative paths) for directory-like artifacts such as skills.
    pub children: Vec<String>,
    pub metadata: Value,
}

impl ConfigArtifact {
    pub fn new(
        kind: ArtifactKind,
        scope: Scope,
        name: impl Into<String>,
        path: impl Into<String>,
        project_path: Option<String>,
    ) -> Self {
        let name = name.into();
        let path = path.into();
        let id = short_hash(&format!(
            "{}|{}|{}|{}|{}",
            kind.as_str(),
            scope.as_str(),
            project_path.as_deref().unwrap_or(""),
            path,
            name
        ));
        Self {
            id,
            kind,
            scope,
            name,
            path,
            project_path,
            status: DiscoveryStatus::Present,
            size_bytes: None,
            modified_at: None,
            description: None,
            error: None,
            children: Vec::new(),
            metadata: Value::Null,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    pub path: String,
    pub name: String,
    pub has_claude_dir: bool,
    pub has_claude_md: bool,
    pub has_mcp_json: bool,
    pub pinned: bool,
    pub discovered: bool,
    pub artifact_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanError {
    pub path: String,
    pub scope: Option<Scope>,
    pub status: DiscoveryStatus,
    pub message: String,
}

/// One settings file's parsed (and secret-masked) content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsLayer {
    pub scope: Scope,
    pub path: String,
    pub project_path: Option<String>,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookEntry {
    pub event: String,
    pub matcher: Option<String>,
    /// `command`, `prompt`, `agent`, ...
    pub hook_type: String,
    pub command: Option<String>,
    pub prompt: Option<String>,
    pub timeout: Option<u64>,
    pub scope: Scope,
    pub source_path: String,
    pub project_path: Option<String>,
    pub details: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerEntry {
    pub name: String,
    pub scope: Scope,
    pub source_path: String,
    pub project_path: Option<String>,
    /// `.mcp.json`, `~/.claude.json`, `~/.claude.json#projects`, `managed-mcp.json`
    pub origin: String,
    /// `stdio`, `http`, `sse`, ...
    pub server_type: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub url: Option<String>,
    /// Masked server config.
    pub config: Value,
    /// Hash of the *unmasked* config, used for conflict detection only.
    #[serde(skip_serializing, default)]
    pub config_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillEntry {
    pub name: String,
    /// `skill` or `command`
    pub kind: ArtifactKind,
    pub description: Option<String>,
    pub scope: Scope,
    pub path: String,
    pub project_path: Option<String>,
    pub status: DiscoveryStatus,
    pub frontmatter: Value,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleEntry {
    pub name: String,
    pub scope: Scope,
    pub path: String,
    pub project_path: Option<String>,
    pub paths: Vec<String>,
    pub conditional: bool,
    pub status: DiscoveryStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEntry {
    pub name: String,
    pub description: Option<String>,
    pub scope: Scope,
    pub path: String,
    pub project_path: Option<String>,
    pub tools: Vec<String>,
    pub model: Option<String>,
    /// name + description both present
    pub valid: bool,
    pub status: DiscoveryStatus,
    pub frontmatter: Value,
}

/// Everything collected for one scope container (global or one project).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub artifacts: Vec<ConfigArtifact>,
    pub hooks: Vec<HookEntry>,
    pub mcp_servers: Vec<McpServerEntry>,
    pub skills: Vec<SkillEntry>,
    pub rules: Vec<RuleEntry>,
    pub agents: Vec<AgentEntry>,
    pub settings_layers: Vec<SettingsLayer>,
    pub errors: Vec<ScanError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalBundle {
    pub config_dir: String,
    pub managed_dir: String,
    pub claude_json_path: String,
    /// `.credentials.json` exists (contents are never read).
    pub credentials_present: bool,
    #[serde(flatten)]
    pub items: Collection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectBundle {
    pub project: ProjectInfo,
    #[serde(flatten)]
    pub items: Collection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConflictKind {
    DuplicateName,
    McpMismatch,
    SettingOverride,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConflictSeverity {
    Info,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictEntry {
    pub scope: Scope,
    pub path: String,
    pub kind: Option<ArtifactKind>,
    pub value: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conflict {
    pub id: String,
    pub kind: ConflictKind,
    pub severity: ConflictSeverity,
    pub key: String,
    pub message: String,
    pub project_path: Option<String>,
    pub entries: Vec<ConflictEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingSource {
    pub scope: Scope,
    pub path: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedSetting {
    pub key: String,
    pub value: Value,
    /// Winning scope (highest precedence contributor).
    pub scope: Scope,
    pub source_path: Option<String>,
    pub confidence: Confidence,
    pub explanation: String,
    /// True when the value is a list-merge of multiple scopes.
    pub merged: bool,
    /// All scopes that define this key, highest precedence first.
    pub sources: Vec<SettingSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDiffRow {
    pub key: String,
    pub user: Option<Value>,
    pub project: Option<Value>,
    pub local: Option<Value>,
    pub differs: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDiff {
    pub project_path: String,
    pub user_path: Option<String>,
    pub project_settings_path: Option<String>,
    pub local_path: Option<String>,
    pub user: Option<Value>,
    pub project: Option<Value>,
    pub local: Option<Value>,
    pub rows: Vec<SettingsDiffRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentChange {
    pub id: i64,
    pub path: String,
    pub kind: String,
    pub ts: String,
    pub hash: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub total_artifacts: usize,
    pub project_count: usize,
    pub skills: usize,
    pub commands: usize,
    pub agents: usize,
    pub rules: usize,
    pub hooks: usize,
    pub mcp_servers: usize,
    pub plugins: usize,
    pub memories: usize,
    pub conflict_count: usize,
    pub error_count: usize,
    pub by_kind: BTreeMap<String, usize>,
    pub by_scope: BTreeMap<String, usize>,
    pub last_scan_at: Option<String>,
    pub recent_changes: Vec<RecentChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSnapshot {
    pub id: String,
    pub scanned_at: String,
    pub duration_ms: u64,
    pub global: GlobalBundle,
    pub projects: Vec<ProjectBundle>,
    pub conflicts: Vec<Conflict>,
    pub errors: Vec<ScanError>,
    pub stats: DashboardStats,
}

impl ScanSnapshot {
    pub fn all_artifacts(&self) -> impl Iterator<Item = &ConfigArtifact> {
        self.global
            .items
            .artifacts
            .iter()
            .chain(self.projects.iter().flat_map(|p| p.items.artifacts.iter()))
    }

    pub fn find_project(&self, path: &str) -> Option<&ProjectBundle> {
        self.projects.iter().find(|p| p.project.path == path)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactContent {
    pub text: String,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Preferences {
    pub theme: String,
    pub watch_on_start: bool,
    pub notifications: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            watch_on_start: false,
            notifications: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeEvent {
    pub path: String,
    pub kind: String,
}

pub fn stable_id(parts: &[&str]) -> String {
    short_hash(&parts.join("|"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enums_serialize_camel_case() {
        assert_eq!(serde_json::to_value(Scope::ProjectLocal).unwrap(), "projectLocal");
        assert_eq!(serde_json::to_value(ArtifactKind::ClaudeMd).unwrap(), "claudeMd");
        assert_eq!(serde_json::to_value(DiscoveryStatus::InvalidJson).unwrap(), "invalidJson");
    }

    #[test]
    fn artifact_fields_camel_case() {
        let a = ConfigArtifact::new(ArtifactKind::Skill, Scope::User, "x", "/p", None);
        let v = serde_json::to_value(&a).unwrap();
        assert!(v.get("projectPath").is_some());
        assert!(v.get("sizeBytes").is_some());
    }
}
