#!/bin/bash
set -e

echo "Init IPFS with server profile..."
ipfs init --profile=server

echo "Starting IPFS daemon in offline mode..."
ipfs daemon --offline &

echo "Waiting for IPFS to be ready on port 5001..."
until nc -w 1 127.0.0.1 5001 </dev/null; do
    echo "IPFS is not ready yet. Retrying in 2 seconds..."
    sleep 2
done

echo "IPFS is ready on port 5001"
exec rollup-init dapp
