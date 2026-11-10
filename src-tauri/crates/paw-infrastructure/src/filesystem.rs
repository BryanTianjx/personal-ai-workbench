//! Managed file store (E2-04): attachment bytes under workspace-scoped
//! directories `attachments/<workspace-id>/<managed-uuid>`.
//!
//! Security contract (data-architecture §6, platform-architecture): every
//! path component is a validated UUID (no traversal), writes go through a
//! temp file + rename, the size cap is enforced before touching disk, and
//! reads verify the canonical path stays inside the workspace directory
//! (symlink-escape refusal).

use std::path::{Path, PathBuf};

use paw_application::ports::file_store::{FileStoreError, ManagedFileStore, StoredFile};
use paw_domain::work_management::WorkspaceId;

use crate::sqlite;

/// Managed store root: `<base>/attachments/<workspace-id>/<managed-uuid>`.
pub struct FsManagedFileStore {
    root: PathBuf,
}

/// Default size cap for one attachment: 50 MiB.
pub const MAX_ATTACHMENT_BYTES: usize = 50 * 1024 * 1024;

impl FsManagedFileStore {
    /// Open the store rooted at `base/attachments`.
    pub fn open(base: &Path) -> Result<Self, sqlite::Error> {
        let root = base.join("attachments");
        std::fs::create_dir_all(&root).map_err(|e| sqlite::Error::Io { source: e })?;
        Ok(Self { root })
    }

    fn workspace_dir(&self, workspace_id: &WorkspaceId) -> Result<PathBuf, FileStoreError> {
        // The workspace id must be a valid UUID: it is a path component.
        uuid::Uuid::parse_str(workspace_id.as_str()).map_err(|_| FileStoreError::InvalidId)?;
        Ok(self.root.join(workspace_id.as_str()))
    }

    fn managed_path(
        &self,
        workspace_id: &WorkspaceId,
        managed_id: &str,
    ) -> Result<PathBuf, FileStoreError> {
        // The managed id must be a valid UUID: it is a path component and
        // cannot be `..`, absolute, or contain separators by construction.
        uuid::Uuid::parse_str(managed_id).map_err(|_| FileStoreError::InvalidId)?;
        Ok(self.workspace_dir(workspace_id)?.join(managed_id))
    }

    /// Verify the file's canonical path stays inside the canonical
    /// workspace directory (symlink-escape refusal).
    fn ensure_within_workspace(
        &self,
        path: &Path,
        workspace_dir: &Path,
    ) -> Result<(), FileStoreError> {
        let canonical_file = std::fs::canonicalize(path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => FileStoreError::NotFound,
            _ => FileStoreError::Invariant(e.to_string()),
        })?;
        let canonical_dir = std::fs::canonicalize(workspace_dir)
            .map_err(|e| FileStoreError::Invariant(e.to_string()))?;
        if !canonical_file.starts_with(&canonical_dir) {
            return Err(FileStoreError::Invariant(
                "path escapes the workspace directory".into(),
            ));
        }
        Ok(())
    }
}

impl ManagedFileStore for FsManagedFileStore {
    fn store(
        &self,
        workspace_id: &WorkspaceId,
        bytes: &[u8],
    ) -> Result<StoredFile, FileStoreError> {
        if bytes.len() > MAX_ATTACHMENT_BYTES {
            return Err(FileStoreError::TooLarge {
                max: MAX_ATTACHMENT_BYTES,
            });
        }
        let dir = self.workspace_dir(workspace_id)?;
        std::fs::create_dir_all(&dir).map_err(|e| FileStoreError::Invariant(e.to_string()))?;

        let managed_id = uuid::Uuid::now_v7().to_string();
        let final_path = dir.join(&managed_id);
        let temp_path = dir.join(format!("{managed_id}.tmp"));

        // Atomic write: temp file, fsync-less rename (desktop single-user;
        // the rename is atomic on the same volume).
        std::fs::write(&temp_path, bytes).map_err(|e| FileStoreError::Invariant(e.to_string()))?;
        std::fs::rename(&temp_path, &final_path)
            .map_err(|e| FileStoreError::Invariant(e.to_string()))?;

        // Hash after the rename so the digest covers the stored bytes.
        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();
        hasher.update(bytes);
        let sha256 = hex_lower(&hasher.finalize());

        Ok(StoredFile {
            managed_id,
            size_bytes: bytes.len() as u64,
            sha256,
        })
    }

    fn open(
        &self,
        workspace_id: &WorkspaceId,
        managed_id: &str,
    ) -> Result<Vec<u8>, FileStoreError> {
        let path = self.managed_path(workspace_id, managed_id)?;
        let dir = self.workspace_dir(workspace_id)?;
        self.ensure_within_workspace(&path, &dir)?;
        std::fs::read(&path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => FileStoreError::NotFound,
            _ => FileStoreError::Invariant(e.to_string()),
        })
    }

    fn remove(&self, workspace_id: &WorkspaceId, managed_id: &str) -> Result<(), FileStoreError> {
        let path = self.managed_path(workspace_id, managed_id)?;
        let dir = self.workspace_dir(workspace_id)?;
        // Absence is already the desired end state (idempotent remove).
        if !path.exists() {
            return Ok(());
        }
        // Refuse to delete anything that resolves outside the workspace
        // (symlink escape) — the guard must come before unlinking.
        self.ensure_within_workspace(&path, &dir)?;
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(FileStoreError::Invariant(e.to_string())),
        }
    }
}

/// Lowercase hex without extra dependencies (mirrors sqlite.rs's helper).
fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(char::from_digit((b >> 4) as u32, 16).unwrap_or('0'));
        out.push(char::from_digit((b & 0x0f) as u32, 16).unwrap_or('0'));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_base(tag: &str) -> PathBuf {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "target/tmp-e2-04-fs-{tag}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn store_open_remove_round_trip() {
        let base = temp_base("roundtrip");
        let store = FsManagedFileStore::open(&base).unwrap();
        let ws = WorkspaceId::new_v7();

        let stored = store.store(&ws, "hello 自磨工坊".as_bytes()).unwrap();
        assert_eq!(stored.size_bytes, "hello 自磨工坊".len() as u64);
        assert_eq!(stored.sha256.len(), 64);

        let bytes = store.open(&ws, &stored.managed_id).unwrap();
        assert_eq!(bytes, "hello 自磨工坊".as_bytes());

        store.remove(&ws, &stored.managed_id).unwrap();
        assert!(matches!(
            store.open(&ws, &stored.managed_id),
            Err(FileStoreError::NotFound)
        ));
        // Remove is idempotent on absence.
        store.remove(&ws, &stored.managed_id).unwrap();
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn traversal_attempts_are_invalid_ids() {
        let base = temp_base("traversal");
        let store = FsManagedFileStore::open(&base).unwrap();
        let ws = WorkspaceId::new_v7();

        for hostile in [
            "../../etc/passwd",
            "C:\\Windows\\evil",
            "abc/def",
            "..",
            ".",
        ] {
            assert!(
                matches!(store.open(&ws, hostile), Err(FileStoreError::InvalidId)),
                "expected InvalidId for {hostile:?}"
            );
        }
        // A UUID workspace id is required too: fake workspace ids cannot
        // smuggle path components.
        assert!(matches!(
            store.open(
                &WorkspaceId::new_v7(),
                uuid::Uuid::now_v7().to_string().as_str()
            ),
            Err(FileStoreError::NotFound)
        ));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn oversize_payloads_are_rejected_before_disk() {
        let base = temp_base("oversize");
        let store = FsManagedFileStore::open(&base).unwrap();
        let ws = WorkspaceId::new_v7();
        let big = vec![0u8; MAX_ATTACHMENT_BYTES + 1];
        assert!(matches!(
            store.store(&ws, &big),
            Err(FileStoreError::TooLarge { .. })
        ));
        // Nothing was written.
        let ws_dir = base.join("attachments").join(ws.as_str());
        assert!(!ws_dir.exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn workspaces_do_not_see_each_others_files() {
        let base = temp_base("scope");
        let store = FsManagedFileStore::open(&base).unwrap();
        let w1 = WorkspaceId::new_v7();
        let w2 = WorkspaceId::new_v7();

        let stored = store.store(&w1, b"secret").unwrap();
        // w2 cannot open w1's file even with the managed id.
        assert!(matches!(
            store.open(&w2, &stored.managed_id),
            Err(FileStoreError::NotFound)
        ));
        let _ = std::fs::remove_dir_all(&base);
    }
}
