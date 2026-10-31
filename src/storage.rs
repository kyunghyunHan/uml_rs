use crate::model::Project;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
const MAX_PROJECT_BYTES: usize = 32 * 1024 * 1024;
pub fn load(path: &Path) -> Result<Project, String> {
    let bytes = read_project(path)?;
    decode(&bytes)
}
fn read_project(path: &Path) -> Result<Vec<u8>, String> {
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
    Ok(bytes)
}
fn decode(bytes: &[u8]) -> Result<Project, String> {
    let mut value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|e| format!("JSON 프로젝트를 읽을 수 없습니다: {e}"))?;
    let version = value
        .get("format_version")
        .and_then(|v| v.as_u64())
        .ok_or("프로젝트 저장 형식 버전이 없거나 올바르지 않습니다")?;
    if !matches!(version, 1..=3) {
        return Err(format!(
            "지원하지 않는 프로젝트 버전: {version} (지원: 1, 2, 3)"
        ));
    }
    if version <= 2 {
        crate::migration::relation_fields_v3(&mut value);
    }
    let mut p: Project = serde_json::from_value(value)
        .map_err(|e| format!("프로젝트 구조를 읽을 수 없습니다: {e}"))?;
    if version == 1 {
        crate::migration::v1_to_v2(&mut p);
    }
    p.format_version = 3;
    p.validate_structure()?;
    p.synchronize_links();
    Ok(p)
}
/// Exact bytes from the read, including legacy versions and external formatting.
#[derive(Clone)]
pub struct FileRevision(Vec<u8>);
pub fn load_tracked(path: &Path) -> Result<(Project, FileRevision), String> {
    let bytes = read_project(path)?;
    Ok((decode(&bytes)?, FileRevision(bytes)))
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    atomic_write_using(path, |f| f.write_all(bytes))
}
fn atomic_write_using(
    path: &Path,
    write: impl FnOnce(&mut fs::File) -> std::io::Result<()>,
) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let tmp = parent.join(format!(".uml-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut f = options.open(&tmp)?;
        if let Ok(metadata) = fs::metadata(path) {
            f.set_permissions(metadata.permissions())?;
        }
        write(&mut f)?;
        f.sync_all()?;
        drop(f);
        fs::rename(&tmp, path)?;
        // Persist the directory entry as well as the contents on Unix filesystems.
        #[cfg(unix)]
        fs::File::open(parent)?.sync_all()?;
        Ok::<_, std::io::Error>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result.map_err(|e| format!("파일 저장 실패 (기존 파일을 확인하세요): {e}"))
}
struct SaveLock(PathBuf);
impl SaveLock {
    fn acquire(path: &Path) -> Result<Self, String> {
        let mut name = path.as_os_str().to_owned();
        name.push(".uml-lock");
        let lock = PathBuf::from(name);
        let mut file = fs::OpenOptions::new()
            .write(true).create_new(true).open(&lock)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    format!("저장 잠금을 확보할 수 없습니다 · {}: {e}. 다른 인스턴스의 저장이 끝난 뒤 다시 시도하세요", lock.display())
                } else {
                    format!("저장 잠금 파일을 만들 수 없습니다 · {}: {e}. 폴더의 접근 권한과 여유 공간을 확인하세요", lock.display())
                }
            })?;
        let guard = Self(lock);
        writeln!(file, "pid={}", std::process::id()).map_err(|e| e.to_string())?;
        Ok(guard)
    }
}
impl Drop for SaveLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn project_bytes(p: &Project) -> Result<Vec<u8>, String> {
    p.validate_structure()?;
    let bytes = serde_json::to_vec_pretty(p).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_PROJECT_BYTES {
        return Err("프로젝트가 32 MiB 제한을 초과했습니다".into());
    }
    Ok(bytes)
}
pub fn save(path: &Path, p: &Project) -> Result<(), String> {
    save_tracked(path, p, None).map(|_| ())
}
/// A short-lived cooperative lock closes the check/rename race between app instances.
/// External tools that ignore the lock must still be coordinated by the user.
pub fn save_tracked(
    path: &Path,
    p: &Project,
    expected: Option<&FileRevision>,
) -> Result<FileRevision, String> {
    p.validate()?;
    let bytes = project_bytes(p)?;
    let _lock = SaveLock::acquire(path)?;
    if let Some(expected) = expected {
        let current = read_project(path).map_err(|e| {
            format!("원본 파일을 확인할 수 없습니다. 다른 이름으로 저장하세요: {e}")
        })?;
        if current != expected.0 {
            return Err("다른 프로그램 또는 인스턴스가 파일을 변경했습니다. 덮어쓰지 않았습니다. 다른 이름으로 저장한 뒤 변경 내용을 비교하세요".into());
        }
    }
    atomic_write(path, &bytes)?;
    Ok(FileRevision(bytes))
}
pub fn save_recovery(path: &Path, p: &Project) -> Result<(), String> {
    atomic_write(path, &project_bytes(p)?)
}
/// Every editing session owns one recovery file; unrelated sessions are never removed.
pub struct RecoveryStore {
    directory: PathBuf,
    own: PathBuf,
}
impl RecoveryStore {
    pub fn new(directory: PathBuf) -> Self {
        let own = directory.join(format!("recovery-{}.umlproj", uuid::Uuid::new_v4()));
        Self { directory, own }
    }
    pub fn save(&self, p: &Project) -> Result<(), String> {
        save_recovery(&self.own, p)
    }
    pub fn clear(&self) -> Result<(), String> {
        Self::discard(&self.own)
    }
    pub fn discard(path: &Path) -> Result<(), String> {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("복구 사본을 삭제할 수 없습니다: {e}")),
        }
    }
    pub fn candidates(&self) -> Vec<PathBuf> {
        let mut files: Vec<_> = fs::read_dir(&self.directory)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p != &self.own
                    && p.is_file()
                    && p.file_name().and_then(|s| s.to_str()).is_some_and(|s| {
                        s == "recovery.umlproj"
                            || (s.starts_with("recovery-") && s.ends_with(".umlproj"))
                    })
            })
            .collect();
        files.sort_by_key(|p| std::cmp::Reverse(fs::metadata(p).and_then(|m| m.modified()).ok()));
        files
    }
}
pub fn config_dir() -> PathBuf {
    let base = std::env::var_os("UML_STUDIO_HOME")
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".uml-studio")))
        .unwrap_or_else(|| std::env::temp_dir().join("uml-studio"));
    let _ = fs::create_dir_all(&base);
    base
}
#[derive(Default, Serialize, Deserialize)]
pub struct Preferences {
    pub recent: Vec<PathBuf>,
    #[serde(default)]
    pub dark_mode: bool,
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
        self.persist();
    }
    pub fn persist(&self) {
        if let Ok(b) = serde_json::to_vec(self) {
            let _ = atomic_write(&config_dir().join("recent.json"), &b);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interrupted_or_full_write_preserves_original_and_removes_temporary_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("project.umlproj");
        fs::write(&path, b"original bytes").unwrap();
        let result = atomic_write_using(&path, |f| {
            f.write_all(b"partial")?;
            Err(std::io::Error::other("injected disk-full/write failure"))
        });
        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"original bytes");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
