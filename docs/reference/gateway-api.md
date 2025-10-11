# Gateway API Reference

The Gateway provides an HTTP API for clients.

## Endpoints

### `POST /api/v1/sign_command`

Requests the oracle to sign a command message after verifying business logic (e.g., payment).

*   **Request Body:**
    ```json
    {
      "message": "<base64-encoded-message>"
    }
    ```
*   **Headers:**
    *   `Authorization`: `Bearer <USER_AUTH_TOKEN>`
*   **Success Response (200 OK):**
    ```json
    {
      "signature": "<base64-encoded-signature>"
    }
    ```
*   **Error Response (4xx/5xx):**
    ```json
    {
      "error": "A descriptive error message."
    }
    ```