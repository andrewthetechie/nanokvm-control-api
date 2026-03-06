#!/bin/bash
# Run DMTF Redfish Service Validator against the API
# Usage: ./validate_redfish.sh [api_url]
# Default api_url: http://localhost:8000

API_URL="${1:-http://localhost:8000}"

echo "Running DMTF Redfish Service Validator against ${API_URL}..."
echo ""

# Run the validator with no auth (our test config disables auth)
python3 -m redfish_service_validator \
    --ip "${API_URL}" \
    --nochkcert \
    --nossl \
    --authtype None \
    --logdir /tmp/redfish-validation \
    2>&1

EXIT_CODE=$?

echo ""
echo "Validation complete. Results in /tmp/redfish-validation/"
echo "Exit code: ${EXIT_CODE}"

exit ${EXIT_CODE}
