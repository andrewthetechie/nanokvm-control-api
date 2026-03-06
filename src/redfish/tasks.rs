//! Redfish Task Service — in-memory task tracking for async operations

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
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
    chrono::Utc::now().to_rfc3339()
}
