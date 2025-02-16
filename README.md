# Wartime Penguin🐧

Guide to walk you through setting up and running the Wartime Penguin NFT project, which uses Cartesi Coprocessor for NFT computation on the Holesky testnet.

## Prerequisites

- MetaMask wallet installed
- Holesky testnet ETH balance
- Node.js and npm installed
- Forge/Foundry installed

## Publishing

1. Navigate to the wartime_penguin directory:
```bash
cd wartime_penguin
```

2. Publish to Holesky testnet:
```bash
cartesi-coprocessor publish --network testnet
```

3. Check the publication status:
```bash
cartesi-coprocessor publish-status --network testnet
```

4. Get the machine hash and task issuer addresses:
```bash
cartesi-coprocessor address-book
```
Note down the testnet addresse and machine hash for the next step.

## Deploying the Smart Contract

1. In a new terminal window, navigate to the contract directory:
```bash
cd contract
```

2. Deploy the contract using Forge:
```bash
forge create --broadcast \
  --rpc-url <your rpc url> \
  --private-key <your private key> \
  ./src/MyContract.sol:MyContract \
  --constructor-args <Testnet_task_issuer> <Machine Hash>
```

Replace the placeholders:
- `<your rpc url>`: Your Holesky RPC URL
- `<your private key>`: Your wallet's private key
- `<Testnet_task_issuer>`: Task issuer address from the previous step
- `<Machine Hash>`: Machine hash from the previous step

## Starting the Application

1. Start the proxy server:
```bash
cd frontend
node proxy-server.js
```
You should see: "Proxy server listening on port 3001"

2. In another terminal window, start the React app:
```bash
npm start
```
The application should open in your default browser.

## Using the Application

1. Connect your wallet:
   - Ensure MetaMask is installed and has Holesky testnet ETH
   - Click "Connect Wallet" in the app
   
2. Generate and mint your NFT:
   - Add your seed phrase
   - Click "Compute NFT"
   - Once computation is complete, click "View NFT"
   - You can now view the NFT metadata and download the car file
   - Click "Mint NFT" to mint your Penguin NFT

Your Penguin NFT will be minted as a GIF, computed using the Cartesi coprocessor!

## Troubleshooting

- Ensure you have sufficient Holesky testnet ETH
- Verify all addresses and hashes are correctly copied from the address book
- Check that both the proxy server and React application are running
- Ensure you're connected to the Holesky testnet in MetaMask

##

Congratulations! You've got yourself a battle-ready penguin powered by the mighty Cartesi coprocessor!
