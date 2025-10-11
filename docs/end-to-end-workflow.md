# Tutorial: End-to-End Workflow with the Example Client

This tutorial demonstrates the project's primary end-to-end workflow using the provided `example-client`.

The previous version of this document showed a complex, multi-step process involving manual transaction creation and interaction with a separate gateway. This was a conceptual guide, not a practical example.

We have replaced that with a fully functional, automated Python client that demonstrates the entire lifecycle of an interaction with the w3b2 toolset. This provides a realistic, hands-on example for developers.

### The Flow at a Glance

The `example-client` service automates the following sequence of on-chain actions through the gRPC gateway:

1.  **Wait for Gateway**: The client waits until the `gateway` service is running and responsive.
2.  **Generate Keypairs**: It creates two in-memory keypairs: one for the `admin_authority` (which also acts as the oracle) and one for the `user_authority`.
3.  **Register Admin**: It calls `PrepareAdminRegisterProfile` and submits the transaction to create a new Admin PDA.
4.  **Register User**: It calls `PrepareUserCreateProfile`, linking the user to the newly created Admin PDA.
5.  **Deposit Funds**: The user calls `PrepareUserDeposit` to add funds to their on-chain profile.
6.  **Dispatch Paid Command**: This is the key step.
    *   The client locally creates a command message (price, timestamp, payload).
    *   It signs this message with the oracle key (`admin_authority`).
    *   It calls `PrepareUserDispatchCommand`, passing the command data and the oracle's signature to the gateway.
    *   The gateway prepares a transaction which the user signs and submits.
7.  **Log Everything**: Throughout this process, the client prints public keys, PDAs, and transaction signatures to the console, providing a clear, real-time log of the entire on-chain interaction.

### How to Run the Example

The best part is that you don't need to run any code manually. The entire demonstration is integrated into the Docker Compose `full` profile.

1.  **Start the Full Stack:**
    From the root of the project, run the following command:
    ```bash
    docker compose --profile full up
    ```

2.  **Observe the Logs:**
    Docker Compose will build and start all the necessary services: the builder, the validator, the deployer, the gateway, and finally, the `example-client`.

    You will see the logs from all services interleaved. Look for the output from the `example-client-1` container. It will look like this, showing the real-time progress of the on-chain workflow:

    ```
    example-client-1  |
    example-client-1  | ============================================================
    example-client-1  |  1. Generating Local Keypairs
    example-client-1  | ============================================================
    example-client-1  | Program ID:           [Program ID]
    example-client-1  | Admin Authority:      [Admin Pubkey]
    example-client-1  | User Authority:       [User Pubkey]
    example-client-1  | Oracle Authority:     [Oracle Pubkey]
    example-client-1  |
    example-client-1  | ============================================================
    example-client-1  |  2. Registering Admin Profile
    example-client-1  | ============================================================
    example-client-1  | Derived Admin PDA:    [Admin PDA]
    example-client-1  | Admin registration successful. Signature: [Transaction Signature]
    example-client-1  |
    example-client-1  | ============================================================
    example-client-1  |  3. Registering User Profile
    example-client-1  | ============================================================
    example-client-1  | Derived User PDA:     [User PDA]
    example-client-1  | User registration successful. Signature: [Transaction Signature]
    ...and so on.
    ```

This automated client serves as a much more powerful and realistic example than the previous conceptual guide. It provides developers with a clear, working model of how to integrate with the `w3b2-gateway` from a client application.