import os
import time
import pytest
import requests

API_URL = os.environ.get("API_URL", "http://localhost:8000")

@pytest.fixture(scope="session", autouse=True)
def wait_for_api():
    max_retries = 30
    for i in range(max_retries):
        try:
            response = requests.get(f"{API_URL}/redfish/v1/")
            if response.status_code == 200:
                return
        except requests.exceptions.ConnectionError:
            pass
        time.sleep(1)
    pytest.fail("API did not start in time")

def test_service_root():
    response = requests.get(f"{API_URL}/redfish/v1/")
    assert response.status_code == 200
    data = response.json()
    assert data["@odata.id"] == "/redfish/v1/"
    assert "Systems" in data
    assert "Managers" in data
    assert "TaskService" in data

def test_systems_collection():
    response = requests.get(f"{API_URL}/redfish/v1/Systems")
    assert response.status_code == 200
    data = response.json()
    assert data["Members@odata.count"] == 1
    assert data["Members"][0]["@odata.id"] == "/redfish/v1/Systems/1"

def test_system_resource():
    response = requests.get(f"{API_URL}/redfish/v1/Systems/1")
    assert response.status_code == 200
    data = response.json()
    assert "PowerState" in data
    assert data["Id"] == "1"

def test_managers_collection():
    response = requests.get(f"{API_URL}/redfish/v1/Managers")
    assert response.status_code == 200
    data = response.json()
    assert data["Members@odata.count"] == 1
    assert data["Members"][0]["@odata.id"] == "/redfish/v1/Managers/1"

def test_manager_resource():
    response = requests.get(f"{API_URL}/redfish/v1/Managers/1")
    assert response.status_code == 200
    data = response.json()
    assert "VirtualMedia" in data

def test_management_power_state():
    # Set to On
    response = requests.put(f"{API_URL}/api/v1/power-state", json={"state": "On"})
    assert response.status_code == 200

    # Get state
    response = requests.get(f"{API_URL}/api/v1/power-state")
    assert response.status_code == 200
    assert response.json()["state"] == "On"

def test_reset_action():
    # ForceOff
    response = requests.post(f"{API_URL}/redfish/v1/Systems/1/Actions/ComputerSystem.Reset", json={"ResetType": "ForceOff"})
    assert response.status_code == 204

    # Verify state is Off
    response = requests.get(f"{API_URL}/redfish/v1/Systems/1")
    assert response.json()["PowerState"] == "Off"

    # ForceOn
    response = requests.post(f"{API_URL}/redfish/v1/Systems/1/Actions/ComputerSystem.Reset", json={"ResetType": "On"})
    assert response.status_code == 204

    # Verify state is On
    response = requests.get(f"{API_URL}/redfish/v1/Systems/1")
    assert response.json()["PowerState"] == "On"

def test_task_service():
    """TaskService endpoint returns valid Redfish resource."""
    response = requests.get(f"{API_URL}/redfish/v1/TaskService")
    assert response.status_code == 200
    data = response.json()
    assert data["@odata.type"] == "#TaskService.v1_2_1.TaskService"
    assert data["Id"] == "TaskService"
    assert data["ServiceEnabled"] is True
    assert "Tasks" in data
    assert data["Tasks"]["@odata.id"] == "/redfish/v1/TaskService/Tasks"

def test_tasks_collection():
    """Tasks collection endpoint is accessible and valid."""
    response = requests.get(f"{API_URL}/redfish/v1/TaskService/Tasks")
    assert response.status_code == 200
    data = response.json()
    assert data["@odata.type"] == "#TaskCollection.TaskCollection"
    assert "Members" in data
    assert "Members@odata.count" in data

def test_insert_media_returns_202_with_task():
    """InsertMedia returns HTTP 202 with Location header and Task body."""
    response = requests.post(
        f"{API_URL}/redfish/v1/Managers/1/VirtualMedia/Cd/Actions/VirtualMedia.InsertMedia",
        json={"Image": "http://example.com/test.iso"},
        allow_redirects=False,
    )
    assert response.status_code == 202, f"Expected 202, got {response.status_code}: {response.text}"
    assert "location" in response.headers
    data = response.json()
    assert data["@odata.type"] == "#Task.v1_7_4.Task"
    assert data["TaskState"] in ["New", "Running", "Completed", "Exception"]

    # Poll the task location until completed (with timeout)
    task_url = f"{API_URL}{response.headers['location']}"
    task_data = None
    for _ in range(30):
        task_response = requests.get(task_url)
        assert task_response.status_code == 200
        task_data = task_response.json()
        if task_data["TaskState"] in ["Completed", "Exception"]:
            break
        time.sleep(0.5)

    # With mock client, download will fail (example.com), that's expected
    # The important thing is the task completed its lifecycle
    assert task_data["TaskState"] in ["Completed", "Exception"]

def test_virtual_media_insert_and_eject():
    """InsertMedia returns 202 (async), then eject works."""
    # Insert media — now returns 202
    response = requests.post(
        f"{API_URL}/redfish/v1/Managers/1/VirtualMedia/Cd/Actions/VirtualMedia.InsertMedia",
        json={"Image": "http://example.com/test.iso"},
        allow_redirects=False,
    )
    assert response.status_code == 202

    # Wait for task to finish
    task_url = f"{API_URL}{response.headers['location']}"
    for _ in range(30):
        task_response = requests.get(task_url)
        task_data = task_response.json()
        if task_data["TaskState"] in ["Completed", "Exception"]:
            break
        time.sleep(0.5)

    # Eject media
    response = requests.post(f"{API_URL}/redfish/v1/Managers/1/VirtualMedia/Cd/Actions/VirtualMedia.EjectMedia")
    assert response.status_code == 204

def test_task_not_found():
    """Requesting a non-existent task returns 404."""
    response = requests.get(f"{API_URL}/redfish/v1/TaskService/Tasks/99999")
    assert response.status_code == 404

def test_boot_override():
    # Patch boot target to Pxe
    response = requests.patch(
        f"{API_URL}/redfish/v1/Systems/1",
        json={"Boot": {"BootSourceOverrideTarget": "Pxe"}}
    )
    assert response.status_code == 200

    # Verify patch applied
    response = requests.get(f"{API_URL}/redfish/v1/Systems/1")
    data = response.json()
    assert data["Boot"]["BootSourceOverrideTarget"] == "Pxe"
