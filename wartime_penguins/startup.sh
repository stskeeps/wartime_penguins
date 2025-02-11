#!/bin/sh
set -e

echo "Init IPFS with server profile..."
ipfs init --profile=server

echo "Starting IPFS daemon in offline mode..."
ipfs daemon --offline &

echo "Waiting for IPFS to listen on port 5001..."
while ! busybox nc -z localhost 5001; do
    sleep 1
done

echo "IPFS is ready on port 5001"
exec rollup-init dapp
