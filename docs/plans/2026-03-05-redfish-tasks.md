# Redfish Task Service Implementation Plan

> **For Antigravity:** REQUIRED WORKFLOW: Use `.agent/workflows/execute-plan.md` to execute this plan in single-flow mode.

**Goal:** Make `InsertMedia` async by returning HTTP 202 with a Redfish Task resource, polling for status via TaskService endpoints.

**Architecture:** In-memory `TaskManager` holds tasks in `Arc<RwLock<HashMap<u64, RedfishTask>>>`. InsertMedia spawns background download via `tokio::spawn`, returns 202 immediately. Three new GET routes expose TaskService, Tasks collection, and individual Task.

**Tech Stack:** Rust, Axum, Tokio, serde. No new crate dependencies.

---

### Task 1: Add TaskManager and Task Models

**Files:**
- Create: `src/redfish/tasks.rs`
- Modify: `src/redfish/models.rs`
- Modify: `src/redfish/mod.rs`

**Step 1: Create `src/redfish/tasks.rs` with TaskManager struct and Task data model**

```rust
//! Redfish Task Service — in-memory task tracking for async operations

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use serde::Serialize;

/// Redfish TaskState enum per Task.v1_7_4
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum TaskState {
    New,
    Starting,
    Running,
    Completed,
    Exception,
    Cancelled,
}

/// A single message attached to a task
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TaskMessage {
    pub message_id: String,
    pub message: String,
    pub severity: String,
}

/// Internal task record
#[derive(Debug, Clone)]
pub struct RedfishTask {
    pub id: u64,
    pub name: String,
    pub task_state: TaskState,
    pub task_status: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub messages: Vec<TaskMessage>,
}

impl RedfishTask {
    /// Serialize to Redfish JSON representation
    pub fn to_json(&self) -> TaskResource {
        TaskResource {
            odata_type: "#Task.v1_7_4.Task",
            odata_id: format!("/redfish/v1/TaskService/Tasks/{}", self.id),
            id: self.id.to_string(),
            name: self.name.clone(),
            task_state: format!("{:?}", self.task_state),
            task_status: self.task_status.clone(),
            start_time: self.start_time.clone(),
            end_time: self.end_time.clone(),
            messages: self.messages.clone(),
        }
    }
}

/// Serializable Redfish Task resource
#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TaskResource {
    #[serde(rename = "@odata.type")]
    pub odata_type: &'static str,
    #[serde(rename = "@odata.id")]
    pub odata_id: String,
    pub id: String,
    pub name: String,
    pub task_state: String,
    pub task_status: String,
    pub start_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    pub messages: Vec<TaskMessage>,
}

/// Serializable Redfish TaskService resource
#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TaskServiceResource {
    #[serde(rename = "@odata.type")]
    pub odata_type: &'static str,
    #[serde(rename = "@odata.id")]
    pub odata_id: &'static str,
    pub id: &'static str,
    pub name: &'static str,
    pub service_enabled: bool,
    pub status: TaskServiceStatus,
    pub tasks: super::models::ResourceLink,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TaskServiceStatus {
    pub state: &'static str,
    pub health: &'static str,
}

/// In-memory task manager
#[derive(Clone)]
pub struct TaskManager {
    tasks: Arc<RwLock<HashMap<u64, RedfishTask>>>,
    next_id: Arc<AtomicU64>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Create a new task in Running state, return its ID
    pub async fn create_task(&self, name: String) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let task = RedfishTask {
            id,
            name,
            task_state: TaskState::Running,
            task_status: "OK".to_string(),
            start_time: now_iso8601(),
            end_time: None,
            messages: vec![],
        };
        self.tasks.write().await.insert(id, task);
        id
    }

    /// Mark a task as completed
    pub async fn complete_task(&self, id: u64) {
        if let Some(task) = self.tasks.write().await.get_mut(&id) {
            task.task_state = TaskState::Completed;
            task.task_status = "OK".to_string();
            task.end_time = Some(now_iso8601());
        }
    }

    /// Mark a task as failed with an error message
    pub async fn fail_task(&self, id: u64, error: String) {
        if let Some(task) = self.tasks.write().await.get_mut(&id) {
            task.task_state = TaskState::Exception;
            task.task_status = "Critical".to_string();
            task.end_time = Some(now_iso8601());
            task.messages.push(TaskMessage {
                message_id: "Base.1.0.GeneralError".to_string(),
                message: error,
                severity: "Critical".to_string(),
            });
        }
    }

    /// Get a task by ID
    pub async fn get_task(&self, id: u64) -> Option<RedfishTask> {
        self.tasks.read().await.get(&id).cloned()
    }

    /// Get all tasks
    pub async fn list_tasks(&self) -> Vec<RedfishTask> {
        self.tasks.read().await.values().cloned().collect()
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

fn now_iso8601() -> String {
    // Use a simple approach — format current time
    // On the target (Linux), this gives proper timestamps
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Format as ISO 8601 — basic approach without chrono dependency
    format!("1970-01-01T00:00:00+00:00") // placeholder, see step 3
}
```

Note: The `now_iso8601()` needs a proper implementation. Since we want to avoid adding chrono, we can use a simple manual formatter or accept the `chrono` dependency. We'll address this in step 3.

**Step 2: Add `chrono` to `Cargo.toml` for proper ISO 8601 timestamps**

Add to `[dependencies]`:
```toml
chrono = { version = "0.4", features = ["serde"] }
```

Then update `now_iso8601()` in `src/redfish/tasks.rs`:
```rust
fn now_iso8601() -> String {
    chrono::Utc::now().to_rfc3339()
}
```

**Step 3: Register the module in `src/redfish/mod.rs`**

Add `pub mod tasks;` to the module declarations.

**Step 4: Add `TaskManager` to `AppState` in `src/state.rs`**

Add field:
```rust
pub task_manager: crate::redfish::tasks::TaskManager,
```

Add `FromRef` impl:
```rust
impl FromRef<AppState> for crate::redfish::tasks::TaskManager {
    fn from_ref(state: &AppState) -> Self {
        state.task_manager.clone()
    }
}
```

**Step 5: Initialize `TaskManager` in `src/main.rs`**

After `let state = state::AppState {`, add:
```rust
task_manager: redfish::tasks::TaskManager::new(),
```

**Step 6: Build and verify compilation**

Run: `cargo build`
Expected: Compiles without errors

**Step 7: Commit**

```bash
git add -A && git commit -m "feat: add TaskManager and Redfish Task data models"
```

---

### Task 2: Add TaskService Routes

**Files:**
- Modify: `src/redfish/mod.rs` (add TaskService route nesting)
- Modify: `src/redfish/tasks.rs` (add route handlers)
- Modify: `src/redfish/models.rs` (add TaskService link to ServiceRoot)

**Step 1: Add route handlers to `src/redfish/tasks.rs`**

Add at the bottom of the file:

```rust
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use crate::auth::RequireAuth;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_task_service))
        .route("/Tasks", get(list_tasks_handler))
        .route("/Tasks/{id}", get(get_task_handler))
}

async fn get_task_service(_auth: RequireAuth) -> Json<TaskServiceResource> {
    Json(TaskServiceResource {
        odata_type: "#TaskService.v1_2_1.TaskService",
        odata_id: "/redfish/v1/TaskService",
        id: "TaskService",
        name: "Task Service",
        service_enabled: true,
        status: TaskServiceStatus {
            state: "Enabled",
            health: "OK",
        },
        tasks: super::models::ResourceLink {
            odata_id: "/redfish/v1/TaskService/Tasks".to_string(),
        },
    })
}

async fn list_tasks_handler(
    State(task_manager): State<TaskManager>,
    _auth: RequireAuth,
) -> Json<super::models::Collection> {
    let tasks = task_manager.list_tasks().await;
    let members: Vec<super::models::ResourceLink> = tasks
        .iter()
        .map(|t| super::models::ResourceLink {
            odata_id: format!("/redfish/v1/TaskService/Tasks/{}", t.id),
        })
        .collect();
    let count = members.len();
    Json(super::models::Collection {
        odata_type: "#TaskCollection.TaskCollection",
        odata_id: "/redfish/v1/TaskService/Tasks".to_string(),
        name: "Task Collection".to_string(),
        members,
        members_count: count,
    })
}

async fn get_task_handler(
    State(task_manager): State<TaskManager>,
    Path(id): Path<u64>,
    _auth: RequireAuth,
) -> Result<Json<TaskResource>, StatusCode> {
    match task_manager.get_task(id).await {
        Some(task) => Ok(Json(task.to_json())),
        None => Err(StatusCode::NOT_FOUND),
    }
}
```

**Step 2: Nest TaskService routes in `src/redfish/mod.rs`**

Update `pub fn routes()`:
```rust
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/", get(service_root))
        .nest("/v1/Systems", systems::routes())
        .nest("/v1/Managers", managers::routes())
        .nest("/v1/TaskService", tasks::routes())
}
```

**Step 3: Add TaskService link to ServiceRoot in `src/redfish/models.rs`**

Add `task_service: ResourceLink` field to `ServiceRoot` struct.

Update `service_root()` in `src/redfish/mod.rs` to include:
```rust
task_service: ResourceLink {
    odata_id: "/redfish/v1/TaskService".to_string(),
},
```

**Step 4: Build and verify**

Run: `cargo build`
Expected: Compiles without errors

**Step 5: Commit**

```bash
git add -A && git commit -m "feat: add TaskService, Tasks collection, and Task GET routes"
```

---

### Task 3: Make InsertMedia Async

**Files:**
- Modify: `src/redfish/managers.rs` (change `insert_media` handler to spawn background task)

**Step 1: Update `insert_media` handler in `src/redfish/managers.rs`**

Change the handler to:
1. Accept `State<TaskManager>` extractor
2. Create a task via `task_manager.create_task()`
3. `tokio::spawn` the download+mount
4. Return HTTP 202 with `Location` header and Task JSON body

```rust
use crate::redfish::tasks::{TaskManager, TaskResource};
use axum::http::header;
use axum::response::IntoResponse;

async fn insert_media(
    State(virtual_media): State<VirtualMediaManager>,
    State(task_manager): State<TaskManager>,
    _auth: RequireAuth,
    Json(payload): Json<InsertMediaRequest>,
) -> impl IntoResponse {
    let task_id = task_manager.create_task(
        format!("Download and mount {}", payload.image)
    ).await;

    let tm = task_manager.clone();
    let vm = virtual_media.clone();
    let image = payload.image.clone();

    tokio::spawn(async move {
        match vm.insert_media(&image).await {
            Ok(()) => {
                tm.complete_task(task_id).await;
                tracing::info!("Task {} completed: mounted {}", task_id, image);
            }
            Err(e) => {
                tm.fail_task(task_id, format!("InsertMedia failed: {}", e)).await;
                tracing::error!("Task {} failed: {}", task_id, e);
            }
        }
    });

    let task = task_manager.get_task(task_id).await.unwrap();
    let location = format!("/redfish/v1/TaskService/Tasks/{}", task_id);

    (
        StatusCode::ACCEPTED,
        [(header::LOCATION, location)],
        Json(task.to_json()),
    )
}
```

**Step 2: Build and verify**

Run: `cargo build`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add -A && git commit -m "feat: make InsertMedia async with Redfish Task tracking"
```

---

### Task 4: Update Integration Tests

**Files:**
- Modify: `tests/integration/test_redfish.py`

**Step 1: Add TaskService tests to `test_redfish.py`**

Add tests for:
- TaskService endpoint returns valid resource
- Tasks collection is accessible
- InsertMedia now returns 202 with Location header
- Polling the task eventually shows Completed

```python
def test_task_service():
    response = requests.get(f"{API_URL}/redfish/v1/TaskService")
    assert response.status_code == 200
    data = response.json()
    assert data["@odata.type"] == "#TaskService.v1_2_1.TaskService"
    assert "Tasks" in data

def test_tasks_collection():
    response = requests.get(f"{API_URL}/redfish/v1/TaskService/Tasks")
    assert response.status_code == 200
    data = response.json()
    assert "Members" in data
    assert "Members@odata.count" in data

def test_insert_media_returns_202_with_task():
    response = requests.post(
        f"{API_URL}/redfish/v1/Managers/1/VirtualMedia/Cd/Actions/VirtualMedia.InsertMedia",
        json={"Image": "http://example.com/test.iso"}
    )
    assert response.status_code == 202
    assert "Location" in response.headers
    data = response.json()
    assert data["@odata.type"] == "#Task.v1_7_4.Task"
    assert data["TaskState"] in ["New", "Running", "Completed"]

    # Poll the task location until completed (with timeout)
    task_url = f"{API_URL}{response.headers['Location']}"
    for _ in range(30):
        task_response = requests.get(task_url)
        assert task_response.status_code == 200
        task_data = task_response.json()
        if task_data["TaskState"] in ["Completed", "Exception"]:
            break
        time.sleep(0.5)

    assert task_data["TaskState"] == "Completed"
```

**Step 2: Update old `test_virtual_media_insert_and_eject` to match new 202 behavior**

The existing test expects `status_code == 204`. Update it to expect 202 and poll for completion before testing eject.

**Step 3: Run integration tests**

Run: `cd tests && docker compose up --build --abort-on-container-exit`
Expected: All tests pass

**Step 4: Commit**

```bash
git add -A && git commit -m "test: update integration tests for async InsertMedia with Task polling"
```

---

### Task 5: Add Redfish Service Validator

**Files:**
- Modify: `tests/integration/requirements.txt` (add redfish-service-validator)
- Create: `tests/integration/test_redfish_validator.py`
- Modify: `tests/integration/Dockerfile.test`

**Step 1: Add `redfish` Python library to requirements**

```
redfish_service_validator
```

**Step 2: Create validator test script**

Create `tests/integration/test_redfish_validator.py` that runs the DMTF Redfish Service Validator against our running API and asserts no critical errors.

Alternatively, add a separate Docker service or script step that runs:
```bash
rf_service_validator --ip http://api:8000 --nochkcert --nossl --authtype None
```

**Step 3: Update `Dockerfile.test` to copy the new test file**

**Step 4: Run full test suite**

Run: `cd tests && docker compose up --build --abort-on-container-exit`
Expected: All tests pass, validator reports no critical schema violations

**Step 5: Commit**

```bash
git add -A && git commit -m "test: add DMTF Redfish Service Validator to integration tests"
```

---

### Task 6: Update ServiceRoot to Include TaskService

**Files:**
- Already handled in Task 2 Step 3

This is a verification step — ensure `GET /redfish/v1/` includes `TaskService` in the response.

---

### Verification Summary

1. **Unit-level**: `cargo build` at each task to ensure compilation
2. **Integration tests**: `cd tests && docker compose up --build --abort-on-container-exit`
   - Existing tests still pass (with updated 202 expectations)
   - New TaskService/Tasks tests pass
   - InsertMedia returns 202, polls to Completed
3. **Compliance**: DMTF Redfish Service Validator reports no critical errors for Task and TaskService resources
