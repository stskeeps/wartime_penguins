import React, { useState } from 'react';
import { ethers } from "ethers";
import { Buffer } from "buffer";
import { decode as cborDecode } from 'cbor2';
import './App.css';

function App() {
  const [walletAddress, setWalletAddress] = useState("");
  const [seed, setSeed] = useState("");
  const [output, setOutput] = useState("");

  const connectWallet = async () => {
    if (window.ethereum) {
      try {
        const provider = new ethers.providers.Web3Provider(window.ethereum);
        await window.ethereum.request({ method: "eth_requestAccounts" });
        const signer = provider.getSigner();
        const address = await signer.getAddress();
        setWalletAddress(address);
        setOutput(`Connected: ${address}`);
      } catch (error) {
        console.error(error);
        setOutput("Error connecting wallet");
      }
    } else {
      alert("Please install MetaMask.");
    }
  };

  const handleMint = async () => {
    if (!walletAddress) {
      alert("Connect your wallet first!");
      return;
    }
    if (!seed) {
      alert("Please enter a seed value!");
      return;
    }
    try {
      const encodedInput = ethers.utils.defaultAbiCoder.encode(
        ["address", "uint256"],
        [walletAddress, seed]
      );
      console.log("Encoded Input:", encodedInput);

      const machineHash = "a24850cd105dd8e24fc827e2295198a111ae19cbc0042b2664607a50b2148450";
      const fixedAddress = "0xA44151489861Fe9e3055d95adC98FbD462B948e7";
      const endpoint = "http://localhost:3001/issue_task";
      console.log("Proxy Endpoint:", endpoint);
      setOutput("Sending request...");

      const response = await fetch(endpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          machineHash,
          fixedAddress,
          input: encodedInput
        })
      });

      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(`Request failed: ${errorText}`);
      }

      const data = await response.json();
      console.log("Response:", data);
      let finalOutput = JSON.stringify(data, null, 2);

      if (data.service_response && data.service_response[1]) {
        const secondResponse = data.service_response[1];
        const digestKey = Object.keys(secondResponse)[0];
        const noticeArray = secondResponse[digestKey][0][1];
        const fullBuffer = Buffer.from(noticeArray);
        const calldata = "0x" + fullBuffer.toString("hex");
        console.log("Full Notice Call (calldata):", calldata);
        finalOutput += "\n\nFull Notice Call (calldata): " + calldata;

        const iface = new ethers.utils.Interface(["function Notice(bytes data)"]);
        const decodedCall = iface.decodeFunctionData("Notice", calldata);
        console.log("Decoded Call:", decodedCall);
        const innerPayload = decodedCall.data;
        finalOutput += "\n\nPayload (hex): " + innerPayload;

        const decodedInner = ethers.utils.defaultAbiCoder.decode(
          ["string", "bytes32"],
          innerPayload
        );
        console.log("Decoded Inner:", decodedInner);
        finalOutput += "\n\nDecoded Notice Payload:";
        finalOutput += "\n  String: " + decodedInner[0];
        finalOutput += "\n  Bytes32: " + decodedInner[1].toString();

        const bytes32Value = decodedInner[1].toString();
        const bytes32No0x = bytes32Value.slice(2);
        console.log("Bytes32 (without 0x):", bytes32No0x);

        const solverEndpoint = `http://localhost:3001/get_preimage/2/${bytes32No0x}`;
        console.log("Solver Endpoint:", solverEndpoint);
        finalOutput += "\n\nSolver Endpoint: " + solverEndpoint;

        const preimageResponse = await fetch(solverEndpoint);
        if (!preimageResponse.ok) {
          const errText = await preimageResponse.text();
          throw new Error("Solver preimage request failed: " + errText);
        }

        const preimageArrayBuffer = await preimageResponse.arrayBuffer();
        const preimageBuffer = Buffer.from(preimageArrayBuffer);
        console.log("Preimage Buffer (hex):", preimageBuffer.toString("hex"));
        finalOutput += "\n\nPreimage Buffer (hex): " + preimageBuffer.toString("hex");

        const decodedPreimage = cborDecode(preimageBuffer);
        console.log("Decoded Preimage (CBOR):", decodedPreimage);
        finalOutput += "\n\nDecoded Preimage (CBOR): " + JSON.stringify(decodedPreimage, null, 2);
      }

      setOutput(finalOutput);
    } catch (error) {
      console.error(error);
      setOutput("Error: " + error.message);
    }
  };

  return (
    <div className="App">
      <div className="container">
        <h1>Mint Your NFT</h1>
        <button onClick={connectWallet}>Connect Wallet</button>
        <input
          type="number"
          placeholder="Enter a seed (uint256)"
          value={seed}
          onChange={(e) => setSeed(e.target.value)}
        />
        <button onClick={handleMint}>Mint</button>
        <pre className="output">{output}</pre>
      </div>
    </div>
  );
}

export default App;
