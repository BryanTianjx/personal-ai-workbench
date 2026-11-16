//! Attachment use cases (E2-04): add, list, read and remove attachments.
//!
//! Flow for `add`: fresh authorization → file-name validation → polymorphic
//! owner verification (typed not-found) → bytes into the managed file store
//! → metadata row; a metadata failure compensates by removing the stored
//! bytes so no orphan is left. Flow for `remove`: metadata row first (an
//! orphaned file is harmless; a dangling record is not), then best-effort
//! byte removal.

use crate::ports::attachment_repository::{
    Attachment, AttachmentOwner, AttachmentRepository, AttachmentRepositoryError,
};
use crate::ports::audit::{AuditOutcome, AuditSink};
use crate::ports::authorization::{Authorizer, ExecutionContext, ProtectedAction};
use crate::ports::file_store::{FileStoreError, ManagedFileStore};
use crate::use_cases::work_management::{WorkManagementError, audit_action, check_authorization};
use paw_domain::work_management::WorkspaceId;

/// Maximum file-name length (mirrors common filesystem limits).
pub const FILE_NAME_MAX: usize = 255;

/// Characters rejected in a stored file name.
///
/// Path separators and `..` would let a name act as a path. `:` is rejected
/// because on Windows it introduces a drive-relative path
/// (`C:payload.sqlite`) or an NTFS alternate data stream. `*` and `?` are
/// illegal in Windows names and act as globs. Shell metacharacters are
/// handled separately: they are legal in real file names, so they are
/// stripped at export time by [`export_file_name`] instead of being refused
/// here.
const FORBIDDEN_FILE_NAME_CHARS: &[char] = &['/', '\\', ':', '*', '?'];

/// Extensions an exported attachment may be handed to the OS handler.
///
/// This is an allowlist, not a denylist: any type not named here is refused
/// rather than launched. It deliberately omits every format an OS handler
/// executes or interprets as code (`exe`, `com`, `scr`, `pif`, `bat`, `cmd`,
/// `msi`, `ps1`, `vbs`, `vbe`, `js`, `jse`, `wsf`, `wsh`, `hta`, `lnk`,
/// `reg`, `jar`, `chm`, `svg` — SVG can carry script — and the legacy
/// macro-capable `doc`/`xls`/`ppt` binaries).
pub const OPENABLE_EXTENSIONS: &[&str] = &[
    "pdf", "txt", "md", "csv", "json", "log", "xml", "yaml", "yml", "toml", "ini", "png", "jpg",
    "jpeg", "gif", "webp", "bmp", "tif", "tiff", "ico", "docx", "xlsx", "pptx", "odt", "ods",
    "odp", "rtf", "epub", "zip", "gz", "tgz", "bz2", "xz", "7z", "tar",
];

/// Validate an original file name: trimmed, non-empty, no path components
/// (`/`, `\\`), no `..`, no `:`, no `*`/`?`, no control characters, within
/// the length limit. The name is stored verbatim (after trim) for display;
/// the managed file's on-disk name is the generated opaque id.
pub fn validate_file_name(name: &str) -> Result<String, WorkManagementError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(WorkManagementError::Validation(
            paw_domain::work_management::ValidationError::EmptyName,
        ));
    }
    if trimmed.chars().count() > FILE_NAME_MAX {
        return Err(WorkManagementError::Validation(
            paw_domain::work_management::ValidationError::NameTooLong { max: FILE_NAME_MAX },
        ));
    }
    let hostile = trimmed.contains("..")
        || trimmed.chars().any(char::is_control)
        || trimmed.chars().any(|c| FORBIDDEN_FILE_NAME_CHARS.contains(&c))
        // Windows silently strips a trailing dot or space, so `a.` and `a`
        // would collide and `a. ` can smuggle a normalised name past a check.
        || trimmed.ends_with('.')
        || trimmed.ends_with(' ');
    if hostile {
        return Err(WorkManagementError::Repository(
            crate::ports::workspace_repository::WorkspaceRepositoryError::Invariant(format!(
                "file name contains forbidden characters: {trimmed:?}"
            )),
        ));
    }
    Ok(trimmed.to_string())
}

/// The lowercased extension of `file_name` if — and only if — it is on
/// [`OPENABLE_EXTENSIONS`]. `None` means the type must not be handed to the
/// OS handler.
pub fn openable_extension(file_name: &str) -> Option<String> {
    let (_, ext) = file_name.rsplit_once('.')?;
    if ext.is_empty() || ext.len() > 8 || !ext.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    let lower = ext.to_ascii_lowercase();
    OPENABLE_EXTENSIONS
        .contains(&lower.as_str())
        .then_some(lower)
}

/// Build the on-disk name used when an attachment is exported to the temp
/// directory for opening.
///
/// The result contains only `[A-Za-z0-9._-]`, so it can never carry a path
/// separator, `:`, or a Windows shell metacharacter — whatever the stored
/// display name holds. The extension is taken from [`openable_extension`],
/// so a type that must not be executed is refused here instead of being
/// written and launched.
pub fn export_file_name(
    attachment_id: &str,
    file_name: &str,
) -> Result<String, WorkManagementError> {
    let ext = openable_extension(file_name).ok_or_else(|| {
        WorkManagementError::Repository(
            crate::ports::workspace_repository::WorkspaceRepositoryError::Invariant(format!(
                "attachment type is not safe to open: {file_name:?}"
            )),
        )
    })?;
    let stem = match file_name.rsplit_once('.') {
        Some((s, _)) => s,
        None => file_name,
    };
    let sanitized = sanitize_stem(stem);
    let id: String = attachment_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(64)
        .collect();
    let id = if id.is_empty() {
        "attachment"
    } else {
        id.as_str()
    };
    Ok(format!("{id}_{sanitized}.{ext}"))
}

/// Reduce a display-name stem to `[A-Za-z0-9._-]`, collapse repeated dots so
/// `..` cannot survive, and cap the length so `id_stem.ext` stays well under
/// `FILE_NAME_MAX`.
fn sanitize_stem(stem: &str) -> String {
    let mut out = String::with_capacity(stem.len());
    for c in stem.chars() {
        let c = if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
            c
        } else {
            '_'
        };
        if c == '.' && out.ends_with('.') {
            continue;
        }
        if out.chars().count() >= 80 {
            break;
        }
        out.push(c);
    }
    let trimmed = out.trim_matches(|c: char| c == '.' || c == '_');
    if trimmed.is_empty() {
        "attachment".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Add an attachment to a project or task inside `workspace_id`.
#[allow(clippy::too_many_arguments)]
pub fn add_attachment(
    repo: &dyn AttachmentRepository,
    store: &dyn ManagedFileStore,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    owner: AttachmentOwner,
    owner_id: &str,
    file_name: &str,
    bytes: &[u8],
) -> Result<Attachment, WorkManagementError> {
    let action = ProtectedAction::CreateAttachment {
        workspace_id: workspace_id.clone(),
    };
    check_authorization(
        authorizer,
        audit_sink,
        ctx,
        &action,
        Some(workspace_id),
        "attachment",
        None,
    )?;
    let file_name = validate_file_name(file_name)?;

    // Owner must exist inside the workspace (typed not-found, not a
    // constraint error).
    repo.verify_owner(workspace_id, owner, owner_id)?;

    // Bytes first (the store enforces size cap and path safety).
    let stored = store.store(workspace_id, bytes).map_err(|e| match e {
        FileStoreError::TooLarge { max } => WorkManagementError::Validation(
            paw_domain::work_management::ValidationError::NameTooLong { max },
        ),
        other => WorkManagementError::Repository(
            crate::ports::workspace_repository::WorkspaceRepositoryError::Invariant(
                other.to_string(),
            ),
        ),
    })?;

    let attachment = Attachment {
        id: uuid::Uuid::now_v7().to_string(),
        workspace_id: workspace_id.clone(),
        owner,
        managed_id: stored.managed_id.clone(),
        owner_id: owner_id.to_string(),
        file_name,
        size_bytes: stored.size_bytes,
        sha256: stored.sha256,
        created_at: 0, // stamped by the adapter
    };
    match repo.insert_attachment(&attachment) {
        Ok(()) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "attachment",
                Some(attachment.id.as_str()),
                AuditOutcome::Allowed,
            );
            Ok(attachment)
        }
        Err(e) => {
            // Compensate: no orphaned stored bytes after a metadata failure.
            let _ = store.remove(workspace_id, &stored.managed_id);
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "attachment",
                None,
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

/// List a record's attachments (metadata only; bytes come from `read`).
pub fn list_attachments(
    repo: &dyn AttachmentRepository,
    workspace_id: &WorkspaceId,
    owner: AttachmentOwner,
    owner_id: &str,
) -> Result<Vec<Attachment>, WorkManagementError> {
    Ok(repo.list_for_owner(workspace_id, owner, owner_id)?)
}

/// Read one attachment's bytes through the managed store. The metadata row
/// supplies the managed id binding and the workspace scope.
pub fn read_attachment(
    repo: &dyn AttachmentRepository,
    store: &dyn ManagedFileStore,
    workspace_id: &WorkspaceId,
    attachment_id: &str,
) -> Result<(Attachment, Vec<u8>), WorkManagementError> {
    let attachment = repo.find_attachment(workspace_id, attachment_id)?.ok_or(
        WorkManagementError::Attachment(AttachmentRepositoryError::AttachmentNotFound),
    )?;
    let bytes = store
        .open(workspace_id, &attachment.managed_id)
        .map_err(|e| {
            WorkManagementError::Repository(
                crate::ports::workspace_repository::WorkspaceRepositoryError::Invariant(
                    e.to_string(),
                ),
            )
        })?;
    Ok((attachment, bytes))
}

/// Remove one attachment: metadata first, then best-effort byte removal.
pub fn remove_attachment(
    repo: &dyn AttachmentRepository,
    store: &dyn ManagedFileStore,
    authorizer: &dyn Authorizer,
    audit_sink: &dyn AuditSink,
    ctx: &ExecutionContext,
    workspace_id: &WorkspaceId,
    attachment_id: &str,
) -> Result<(), WorkManagementError> {
    let action = ProtectedAction::DeleteAttachment {
        workspace_id: workspace_id.clone(),
    };
    check_authorization(
        authorizer,
        audit_sink,
        ctx,
        &action,
        Some(workspace_id),
        "attachment",
        Some(attachment_id),
    )?;
    // Look the metadata up first: the managed-file id lives on the row, and
    // the typed not-found must fire (audited) before any deletion.
    let managed_id = match repo.find_attachment(workspace_id, attachment_id)? {
        Some(a) => a.managed_id,
        None => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "attachment",
                Some(attachment_id),
                AuditOutcome::Error,
            );
            return Err(WorkManagementError::Attachment(
                AttachmentRepositoryError::AttachmentNotFound,
            ));
        }
    };
    match repo.delete_attachment(workspace_id, attachment_id) {
        Ok(()) => {
            // Best-effort byte removal; an orphaned file is harmless.
            let _ = store.remove(workspace_id, &managed_id);
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "attachment",
                Some(attachment_id),
                AuditOutcome::Allowed,
            );
            Ok(())
        }
        Err(e) => {
            audit_action(
                audit_sink,
                ctx,
                &action,
                Some(workspace_id),
                "attachment",
                Some(attachment_id),
                AuditOutcome::Error,
            );
            Err(e.into())
        }
    }
}

// NewAttachment is consumed by adapter-level tests; the use case builds the
// validated Attachment directly. Keep the type imported for the public API.
#[allow(unused_imports)]
pub use crate::ports::attachment_repository::NewAttachment as _NewAttachment;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::file_store::StoredFile;
    use crate::use_cases::work_management::test_support::{FakeAudit, FakeAuthorizer, ctx};
    use paw_domain::work_management::{ProjectId, WorkspaceId};
    use std::cell::RefCell;

    /// In-memory metadata fake with owner verification against a fixed set.
    struct FakeAttachmentRepo {
        pub rows: RefCell<Vec<Attachment>>,
        pub known_projects: Vec<String>,
    }

    impl FakeAttachmentRepo {
        fn new(known_projects: Vec<String>) -> Self {
            Self {
                rows: RefCell::new(Vec::new()),
                known_projects,
            }
        }
    }

    impl AttachmentRepository for FakeAttachmentRepo {
        fn insert_attachment(
            &self,
            attachment: &Attachment,
        ) -> Result<(), AttachmentRepositoryError> {
            self.verify_owner(
                &attachment.workspace_id,
                attachment.owner,
                &attachment.owner_id,
            )?;
            self.rows.borrow_mut().push(attachment.clone());
            Ok(())
        }

        fn find_attachment(
            &self,
            workspace_id: &WorkspaceId,
            attachment_id: &str,
        ) -> Result<Option<Attachment>, AttachmentRepositoryError> {
            Ok(self
                .rows
                .borrow()
                .iter()
                .find(|a| a.id == attachment_id && a.workspace_id == *workspace_id)
                .cloned())
        }

        fn list_for_owner(
            &self,
            workspace_id: &WorkspaceId,
            owner: AttachmentOwner,
            owner_id: &str,
        ) -> Result<Vec<Attachment>, AttachmentRepositoryError> {
            Ok(self
                .rows
                .borrow()
                .iter()
                .filter(|a| {
                    a.workspace_id == *workspace_id && a.owner == owner && a.owner_id == owner_id
                })
                .cloned()
                .collect())
        }

        fn count_for_workspace(
            &self,
            workspace_id: &WorkspaceId,
        ) -> Result<i64, AttachmentRepositoryError> {
            Ok(self
                .rows
                .borrow()
                .iter()
                .filter(|a| a.workspace_id == *workspace_id)
                .count() as i64)
        }

        fn delete_attachment(
            &self,
            workspace_id: &WorkspaceId,
            attachment_id: &str,
        ) -> Result<(), AttachmentRepositoryError> {
            let mut rows = self.rows.borrow_mut();
            let before = rows.len();
            rows.retain(|a| !(a.id == attachment_id && a.workspace_id == *workspace_id));
            if rows.len() == before {
                Err(AttachmentRepositoryError::AttachmentNotFound)
            } else {
                Ok(())
            }
        }

        fn verify_owner(
            &self,
            _workspace_id: &WorkspaceId,
            owner: AttachmentOwner,
            owner_id: &str,
        ) -> Result<(), AttachmentRepositoryError> {
            match owner {
                AttachmentOwner::Project if self.known_projects.iter().any(|p| p == owner_id) => {
                    Ok(())
                }
                _ => Err(AttachmentRepositoryError::OwnerNotFound),
            }
        }
    }

    /// In-memory file store fake.
    #[derive(Default)]
    struct FakeStore {
        pub files: RefCell<Vec<(String, Vec<u8>)>>,
        pub seq: std::cell::Cell<u64>,
    }

    impl ManagedFileStore for FakeStore {
        fn store(
            &self,
            _workspace_id: &WorkspaceId,
            bytes: &[u8],
        ) -> Result<StoredFile, FileStoreError> {
            let id = format!("managed-{}", self.seq.get());
            self.seq.set(self.seq.get() + 1);
            self.files.borrow_mut().push((id.clone(), bytes.to_vec()));
            Ok(StoredFile {
                managed_id: id,
                size_bytes: bytes.len() as u64,
                sha256: "0".repeat(64),
            })
        }

        fn open(
            &self,
            _workspace_id: &WorkspaceId,
            managed_id: &str,
        ) -> Result<Vec<u8>, FileStoreError> {
            self.files
                .borrow()
                .iter()
                .find(|(id, _)| id == managed_id)
                .map(|(_, b)| b.clone())
                .ok_or(FileStoreError::NotFound)
        }

        fn remove(
            &self,
            _workspace_id: &WorkspaceId,
            managed_id: &str,
        ) -> Result<(), FileStoreError> {
            let mut files = self.files.borrow_mut();
            files.retain(|(id, _)| id != managed_id);
            Ok(())
        }
    }

    #[test]
    fn add_attachment_requires_existing_owner_and_audits() {
        let project = ProjectId::new_v7();
        let repo = FakeAttachmentRepo::new(vec![project.as_str().to_string()]);
        let store = FakeStore::default();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        let ws = WorkspaceId::new_v7();

        // Unknown owner: typed not-found before any bytes are stored.
        let result = add_attachment(
            &repo,
            &store,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            AttachmentOwner::Project,
            ProjectId::new_v7().as_str(),
            "spec.md",
            b"bytes",
        );
        assert!(matches!(
            result,
            Err(WorkManagementError::Attachment(
                AttachmentRepositoryError::OwnerNotFound
            ))
        ));
        assert!(store.files.borrow().is_empty());

        let attachment = add_attachment(
            &repo,
            &store,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            AttachmentOwner::Project,
            project.as_str(),
            "  需求说明.md  ",
            b"bytes",
        )
        .unwrap();
        assert_eq!(attachment.file_name, "需求说明.md");
        assert_eq!(store.files.borrow().len(), 1);
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("attachment.create".into(), AuditOutcome::Allowed))
        );
        assert_eq!(
            list_attachments(&repo, &ws, AttachmentOwner::Project, project.as_str())
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn file_name_validation_blocks_path_components() {
        for hostile in ["../evil", "a/b", "a\\b", "..", "bad\nname"] {
            assert!(
                validate_file_name(hostile).is_err(),
                "expected rejection for {hostile:?}"
            );
        }
        assert!(validate_file_name("  ok-name_v1.md  ").is_ok());
    }

    #[test]
    fn file_name_validation_blocks_drive_relative_and_glob_chars() {
        // `:` enables a Windows drive-relative path or an ADS; `*`/`?` are
        // illegal in Windows names and act as globs; a trailing dot/space is
        // silently stripped by Windows and would collide with the bare name.
        for hostile in ["C:payload.sqlite", "a:b.txt", "a?.txt", "a*.txt", "name."] {
            assert!(
                validate_file_name(hostile).is_err(),
                "expected rejection for {hostile:?}"
            );
        }
        // A trailing space is removed by the leading trim, which is the safe
        // normalisation Windows applies anyway; the result must be the bare
        // stem, never a name that still ends in whitespace.
        assert_eq!(validate_file_name("name ").unwrap(), "name");
    }

    #[test]
    fn file_name_validation_still_accepts_ordinary_names_with_shell_metachars() {
        // These are legal file names; sanitising for the shell is the export
        // path's job, so add-time validation must not break them.
        for ok in [
            "Q1 & Q2 report.pdf",
            "a-b_c.v2.docx",
            "notes (final).md",
            "50% done.txt",
        ] {
            assert!(
                validate_file_name(ok).is_ok(),
                "expected acceptance for {ok:?}"
            );
        }
    }

    #[test]
    fn openable_extension_allows_inert_types_only() {
        assert_eq!(openable_extension("report.pdf").as_deref(), Some("pdf"));
        assert_eq!(openable_extension("SHOT.PNG").as_deref(), Some("png"));
        assert_eq!(openable_extension("archive.tar.gz").as_deref(), Some("gz"));
        for refuse in [
            "payload.exe",
            "payload.bat",
            "payload.cmd",
            "payload.ps1",
            "payload.lnk",
            "payload.hta",
            "payload.js",
            "payload.vbs",
            "payload.scr",
            "payload.msi",
            "payload.svg",
            "payload.docm",
            "payload.jar",
            "noextension",
            "trailingdot.",
        ] {
            assert!(
                openable_extension(refuse).is_none(),
                "expected refusal for {refuse:?}"
            );
        }
    }

    #[test]
    fn export_file_name_cannot_carry_a_shell_metacharacter() {
        // The stored display name is attacker-controlled; the exported name
        // must be inert before it reaches any command line.
        for name in [
            "q1&calc&q2.txt",
            "a|b.txt",
            "c^d.txt",
            "e(f).txt",
            "g;h.txt",
            "i%PATH%j.txt",
            "k\"l.txt",
            "m n & o.pdf",
            "..\\..\\evil.pdf",
        ] {
            let exported = export_file_name("0193f0aa-1111-7000-8000-000000000000", name).unwrap();
            for bad in [
                '&', '|', '^', '(', ')', ';', '%', '"', '!', '/', '\\', ':', '*', '?',
            ] {
                assert!(
                    !exported.contains(bad),
                    "exported name {exported:?} still contains {bad:?} (from {name:?})"
                );
            }
            assert!(
                !exported.contains(".."),
                "exported {exported:?} contains .."
            );
            assert!(
                !exported.starts_with('.'),
                "exported {exported:?} starts with ."
            );
        }
        assert_eq!(
            export_file_name("0193f0aa-1111-7000-8000-000000000000", "q1&calc&q2.txt").unwrap(),
            "0193f0aa-1111-7000-8000-000000000000_q1_calc_q2.txt"
        );
    }

    #[test]
    fn export_file_name_refuses_executable_types() {
        for refuse in [
            "payload.bat",
            "payload.exe",
            "payload.ps1",
            "payload.lnk",
            "payload",
        ] {
            assert!(
                export_file_name("id", refuse).is_err(),
                "expected refusal for {refuse:?}"
            );
        }
    }

    #[test]
    fn remove_attachment_deletes_metadata_then_bytes() {
        let project = ProjectId::new_v7();
        let repo = FakeAttachmentRepo::new(vec![project.as_str().to_string()]);
        let store = FakeStore::default();
        let authz = FakeAuthorizer::granting("local-user");
        let audit = FakeAudit::default();
        let ws = WorkspaceId::new_v7();

        let attachment = add_attachment(
            &repo,
            &store,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            AttachmentOwner::Project,
            project.as_str(),
            "a.txt",
            b"data",
        )
        .unwrap();

        remove_attachment(
            &repo,
            &store,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            &attachment.id,
        )
        .unwrap();
        assert!(repo.rows.borrow().is_empty());
        assert!(store.files.borrow().is_empty());
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("attachment.delete".into(), AuditOutcome::Allowed))
        );

        // Unknown attachment: typed not-found, audited as error.
        let result = remove_attachment(
            &repo,
            &store,
            &authz,
            &audit,
            &ctx("local-user"),
            &ws,
            "ghost",
        );
        assert!(matches!(
            result,
            Err(WorkManagementError::Attachment(
                AttachmentRepositoryError::AttachmentNotFound
            ))
        ));
        assert_eq!(
            audit.events.borrow().last(),
            Some(&("attachment.delete".into(), AuditOutcome::Error))
        );
    }
}
