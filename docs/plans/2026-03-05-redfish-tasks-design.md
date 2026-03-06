# Redfish Task Service for Async InsertMedia

## Problem

The `VirtualMedia.InsertMedia` action blocks the HTTP response until the ISO download + mount completes. For large ISOs this causes client timeouts.

## Solution

Make `InsertMedia` asynchronous using the Redfish Task Service pattern (Task v1_7_4, TaskService v1_2_1). The action returns HTTP 202 immediately, spawns the download in the background, and the client polls a task resource for status.

## Scope

InsertMedia only. No other operations use the task system. The API remains limited to boot overrides, power control, and virtual media.

## Architecture

### TaskManager

An in-memory `TaskManager` struct added to `AppState`:

```rust
struct TaskManager {
    tasks: Arc<RwLock<HashMap<u64, RedfishTask>>>,
    next_id: Arc<AtomicU64>,
}
```

### RedfishTask Data Model

Per Redfish Task v1_7_4:

```rust
struct RedfishTask {
    id: u64,
    name: String,
    task_state: TaskState,      // New, Running, Completed, Exception, Cancelled
    task_status: String,        // "OK", "Warning", "Critical"
    start_time: String,         // ISO 8601
    end_time: Option<String>,
    messages: Vec<TaskMessage>,
    percent_complete: Option<u32>,
}
```

TaskState values: `New`, `Starting`, `Running`, `Completed`, `Exception`, `Cancelled`.

### Data Flow

1. Client POSTs `InsertMedia` with `{"Image": "https://..."}`
2. Handler creates task entry (`TaskState::Running`), spawns `tokio::spawn` for download+mount
3. Returns **HTTP 202 Accepted** with `Location` header → `/redfish/v1/TaskService/Tasks/{id}` and Task JSON body
4. Background task updates state to `Completed` or `Exception`
5. Client polls `GET /redfish/v1/TaskService/Tasks/{id}`

### New Routes

| Method | URI | Purpose |
|--------|-----|---------|
| GET | `/redfish/v1/TaskService` | TaskService singleton |
| GET | `/redfish/v1/TaskService/Tasks` | Task collection |
| GET | `/redfish/v1/TaskService/Tasks/{id}` | Individual task |

`ServiceRoot` gets a `TaskService` link added.

### Error Handling

- Download failure → `TaskState::Exception`, `TaskStatus: "Critical"`, with message
- Task not found → HTTP 404
- Tasks kept in memory (no persistence needed for this BMC)

## Verification

Use DMTF **Redfish-Service-Validator** to validate all resources against CSDL schema. Add as a step in Docker Compose integration tests.
