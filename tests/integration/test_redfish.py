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

def test_virtual_media_insert_and_eject():
    # Insert media
    response = requests.post(
        f"{API_URL}/redfish/v1/Managers/1/VirtualMedia/Cd/Actions/VirtualMedia.InsertMedia",
        json={"Image": "http://example.com/test.iso"}
    )
    assert response.status_code == 204

    # Eject media
    response = requests.post(f"{API_URL}/redfish/v1/Managers/1/VirtualMedia/Cd/Actions/VirtualMedia.EjectMedia")
    assert response.status_code == 204

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
