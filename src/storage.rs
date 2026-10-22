use crate::model::Project;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
const MAX_PROJECT_BYTES: usize = 32 * 1024 * 1024;
pub fn load(path: &Path) -> Result<Project, String> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .and_then(|f| {
            f.take((MAX_PROJECT_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
        })
        .map_err(|e| format!("파일 열기 실패: {e}"))?;
    if bytes.len() > MAX_PROJECT_BYTES {
        return Err("프로젝트가 32 MiB 제한을 초과했습니다".into());
    }
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|e| format!("JSON 프로젝트를 읽을 수 없습니다: {e}"))?;
    let version = value
        .get("format_version")
        .and_then(|v| v.as_u64())
        .ok_or("프로젝트 저장 형식 버전이 없거나 올바르지 않습니다")?;
    if version != 1 {
        return Err(format!("지원하지 않는 프로젝트 버전: {version} (지원: 1)"));
    }
    let p: Project = serde_json::from_value(value)
        .map_err(|e| format!("프로젝트 구조를 읽을 수 없습니다: {e}"))?;
    p.validate()?;
    Ok(p)
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let tmp = parent.join(format!(".uml-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;

        fs::rename(&tmp, path)?;
        Ok::<_, std::io::Error>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result.map_err(|e| format!("파일 저장 실패 (기존 파일을 확인하세요): {e}"))
}
pub fn save(path: &Path, p: &Project) -> Result<(), String> {
    p.validate()?;
    let bytes = serde_json::to_vec_pretty(p).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_PROJECT_BYTES {
        return Err("프로젝트가 32 MiB 제한을 초과했습니다".into());
    }
    atomic_write(path, &bytes)
}
pub fn config_dir() -> PathBuf {
    let base = std::env::var_os("UML_STUDIO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".uml-studio")))
        .unwrap_or_else(std::env::temp_dir);
    let _ = fs::create_dir_all(&base);
    base
}
#[derive(Default, Serialize, Deserialize)]
pub struct Preferences {
    pub recent: Vec<PathBuf>,
}
impl Preferences {
    pub fn load() -> Self {
        fs::read(config_dir().join("recent.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }
    pub fn remember(&mut self, p: &Path) {
        self.recent.retain(|a| a != p);
        self.recent.insert(0, p.to_owned());
        self.recent.truncate(12);
        if let Ok(b) = serde_json::to_vec(self) {
            let _ = atomic_write(&config_dir().join("recent.json"), &b);
        }
    }
}
