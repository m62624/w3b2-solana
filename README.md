1. Запусти локальный валидатор:

   ```bash
   solana-test-validator --reset
   ```

   он поднимет RPC на `http://127.0.0.1:8899`.

2. В другом терминале укажи CLI, что нужно ходить в локалку:

   ```bash
   solana config set --url http://127.0.0.1:8899
   ```

3. Проверь:

   ```bash
   solana config get
   ```

   должно быть:

   ```
   RPC URL: http://127.0.0.1:8899
   ```

4. Сразу можно закинуть себе токенов:

   ```bash
   solana airdrop 10
   solana balance
   ```

 `solana program deploy`, `solana balance`  будут работать на локальной цепочке без риска для mainnet.

