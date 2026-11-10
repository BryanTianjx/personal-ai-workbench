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

/// Validate an original file name: trimmed, non-empty, no path components
/// (`/`, `\`), no `..`, no control characters, within the length limit.
/// The name is stored verbatim (after trim) for display; the managed file's
/// on-disk name is the generated opaque id.
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
    let hostile = trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
        || trimmed.chars().any(char::is_control);
    if hostile {
        return Err(WorkManagementError::Repository(
            crate::ports::workspace_repository::WorkspaceRepositoryError::Invariant(format!(
                "file name contains forbidden characters: {trimmed:?}"
            )),
        ));
    }
    Ok(trimmed.to_string())
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
