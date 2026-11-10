//! E2-04 integration tests: attachments end to end on real files — managed
//! store round-trip, workspace scoping, polymorphic-owner verification, and
//! the deletion guards (project/task/workspace refuse while attachments
//! remain; no silent cascades into stored bytes).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use paw_application::ports::attachment_repository::{
    Attachment, AttachmentOwner, AttachmentRepository, AttachmentRepositoryError,
};
use paw_application::ports::file_store::{FileStoreError, ManagedFileStore};
use paw_application::ports::task_repository::TaskRepository;
use paw_application::ports::workspace_repository::WorkspaceRepository;
use paw_application::{Project, ProjectId, Task, TaskId, Workspace, WorkspaceId};
use paw_infrastructure::filesystem::FsManagedFileStore;
use paw_infrastructure::repositories::attachment_repository::SqliteAttachmentRepository;
use paw_infrastructure::repositories::task_repository::SqliteTaskRepository;
use paw_infrastructure::repositories::workspace_repository::SqliteWorkspaceRepository;
use paw_infrastructure::sqlite;

fn migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations/sqlite")
}

fn temp_dir(tag: &str) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "target/tmp-e2-04-{tag}-{seq}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

struct TempDirGuard {
    dir: PathBuf,
}

impl TempDirGuard {
    fn new(dir: PathBuf) -> Self {
        Self { dir }
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn open_migrated(db: &Path) {
    let mut conn = sqlite::open(db).unwrap();
    sqlite::migrate(&mut conn, &migrations_dir()).unwrap();
    drop(conn);
}

#[test]
fn attachment_round_trip_with_scope_and_owner_checks() {
    let dir = temp_dir("roundtrip");
    let _guard = TempDirGuard::new(dir.clone());
    let db = dir.join("app.sqlite");
    open_migrated(&db);

    let workspaces = SqliteWorkspaceRepository::open(&db).unwrap();
    let attachments = SqliteAttachmentRepository::open(&db).unwrap();
    let store = FsManagedFileStore::open(&dir).unwrap();

    let w1 = Workspace {
        id: WorkspaceId::new_v7(),
        name: "主空间".into(),
    };
    workspaces.insert_workspace(&w1).unwrap();
    let w2 = Workspace {
        id: WorkspaceId::new_v7(),
        name: "别处".into(),
    };
    workspaces.insert_workspace(&w2).unwrap();
    let project = Project::new(ProjectId::new_v7(), w1.id.clone(), "附件项目").unwrap();
    workspaces.insert_project(&w1.id, &project).unwrap();

    // Store bytes then insert metadata bound to the managed id.
    let stored = store.store(&w1.id, "需求文档内容".as_bytes()).unwrap();
    let attachment = Attachment {
        id: uuid::Uuid::now_v7().to_string(),
        workspace_id: w1.id.clone(),
        owner: AttachmentOwner::Project,
        owner_id: project.id.as_str().to_string(),
        managed_id: stored.managed_id.clone(),
        file_name: "需求文档.md".into(),
        size_bytes: stored.size_bytes,
        sha256: stored.sha256.clone(),
        created_at: 0,
    };
    attachments.insert_attachment(&attachment).unwrap();

    // Metadata listing is owner- and workspace-scoped.
    assert_eq!(
        attachments
            .list_for_owner(&w1.id, AttachmentOwner::Project, project.id.as_str())
            .unwrap()
            .len(),
        1
    );
    assert!(
        attachments
            .find_attachment(&w2.id, &attachment.id)
            .unwrap()
            .is_none()
    );

    // Bytes open through the metadata's managed id inside the same scope;
    // another workspace cannot open them even with the managed id.
    let (loaded, bytes) = {
        let meta = attachments
            .find_attachment(&w1.id, &attachment.id)
            .unwrap()
            .unwrap();
        let bytes = store.open(&w1.id, &meta.managed_id).unwrap();
        (meta, bytes)
    };
    assert_eq!(loaded.file_name, "需求文档.md");
    assert_eq!(bytes, "需求文档内容".as_bytes());
    assert!(matches!(
        store.open(&w2.id, &stored.managed_id),
        Err(FileStoreError::NotFound)
    ));

    // Unknown owner: typed not-found before any metadata write.
    assert_eq!(
        attachments.verify_owner(&w1.id, AttachmentOwner::Task, "no-such-task"),
        Err(AttachmentRepositoryError::OwnerNotFound)
    );
    assert_eq!(
        attachments.verify_owner(&w1.id, AttachmentOwner::Project, "no-such-project"),
        Err(AttachmentRepositoryError::OwnerNotFound)
    );
    // An owner id from another workspace is equally not found here.
    let foreign_project = Project::new(ProjectId::new_v7(), w2.id.clone(), "别处项目").unwrap();
    workspaces.insert_project(&w2.id, &foreign_project).unwrap();
    assert_eq!(
        attachments.verify_owner(
            &w1.id,
            AttachmentOwner::Project,
            foreign_project.id.as_str()
        ),
        Err(AttachmentRepositoryError::OwnerNotFound)
    );

    // Removing the metadata row is the authoritative deletion; the bytes
    // removal goes through the managed id afterwards.
    attachments
        .delete_attachment(&w1.id, &attachment.id)
        .unwrap();
    store.remove(&w1.id, &loaded.managed_id).unwrap();
    assert!(matches!(
        store.open(&w1.id, &stored.managed_id),
        Err(FileStoreError::NotFound)
    ));
}

#[test]
fn owner_deletion_refuses_while_attachments_remain() {
    let dir = temp_dir("guards");
    let _guard = TempDirGuard::new(dir.clone());
    let db = dir.join("app.sqlite");
    open_migrated(&db);

    let workspaces = SqliteWorkspaceRepository::open(&db).unwrap();
    let tasks = SqliteTaskRepository::open(&db).unwrap();
    let attachments = SqliteAttachmentRepository::open(&db).unwrap();
    let store = FsManagedFileStore::open(&dir).unwrap();

    let ws = Workspace {
        id: WorkspaceId::new_v7(),
        name: "守卫空间".into(),
    };
    workspaces.insert_workspace(&ws).unwrap();
    let project = Project::new(ProjectId::new_v7(), ws.id.clone(), "有附件的项目").unwrap();
    workspaces.insert_project(&ws.id, &project).unwrap();
    let task = Task::new(
        TaskId::new_v7(),
        ws.id.clone(),
        Some(project.id.clone()),
        "有附件的任务",
        None,
    )
    .unwrap();
    tasks.insert_task(&task).unwrap();

    for owner_id in [
        project.id.as_str().to_string(),
        task.id.as_str().to_string(),
    ] {
        let owner = if owner_id == project.id.as_str() {
            AttachmentOwner::Project
        } else {
            AttachmentOwner::Task
        };
        let stored = store.store(&ws.id, b"payload").unwrap();
        attachments
            .insert_attachment(&Attachment {
                id: uuid::Uuid::now_v7().to_string(),
                workspace_id: ws.id.clone(),
                owner,
                owner_id: owner_id.clone(),
                managed_id: stored.managed_id.clone(),
                file_name: "data.bin".into(),
                size_bytes: stored.size_bytes,
                sha256: stored.sha256.clone(),
                created_at: 0,
            })
            .unwrap();
    }

    // Project deletion refuses while its attachment remains.
    let project_result = workspaces.delete_project(&ws.id, &project.id).unwrap_err();
    assert!(
        project_result.to_string().contains("attachment"),
        "expected attachment guard, got {project_result}"
    );
    // Task deletion likewise.
    let task_result = tasks.delete_task(&ws.id, &task.id).unwrap_err();
    assert!(
        task_result.to_string().contains("attachment"),
        "expected attachment guard, got {task_result}"
    );
    // Workspace deletion refuses while any attachment remains anywhere.
    let ws_result = workspaces.delete_workspace(&ws.id).unwrap_err();
    assert!(
        ws_result.to_string().contains("attachment"),
        "expected attachment guard, got {ws_result}"
    );

    // After removing both attachments the guards open up.
    let all: Vec<Attachment> = [
        attachments
            .list_for_owner(&ws.id, AttachmentOwner::Project, project.id.as_str())
            .unwrap(),
        attachments
            .list_for_owner(&ws.id, AttachmentOwner::Task, task.id.as_str())
            .unwrap(),
    ]
    .concat();
    for attachment in all {
        attachments
            .delete_attachment(&ws.id, &attachment.id)
            .unwrap();
        store.remove(&ws.id, &attachment.managed_id).unwrap();
    }
    // Child order: task first (it references the project), then project,
    // then the workspace.
    tasks.delete_task(&ws.id, &task.id).unwrap();
    workspaces.delete_project(&ws.id, &project.id).unwrap();
    workspaces.delete_workspace(&ws.id).unwrap();
    assert!(workspaces.find_workspace(&ws.id).unwrap().is_none());
}
