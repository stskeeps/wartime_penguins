import React, { useState } from 'react';
import { ethers } from "ethers";
import { Buffer } from "buffer";
import { decode as cborDecode } from 'cbor2';
import { createHelia } from 'helia';
import { CID } from 'multiformats/cid';
import { sha256 } from 'multiformats/hashes/sha2';
import { CarWriter } from '@ipld/car/writer';
import { car } from '@helia/car'
import './App.css';

async function carWriterOutToBlob (carReaderIterable) {
    const parts = []
    for await (const part of carReaderIterable) {
      parts.push(part)
    }
    return new Blob(parts, { type: 'application/car' })
  }

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

      const machineHash = "e33405834bb3bd002e08d68ffb0f5748fb682ad2c0bda78edeea6506b74374f5";
      const fixedAddress = "0xA44151489861Fe9e3055d95adC98FbD462B948e7";
      const endpoint = "http://localhost:3001/issue_task";
      console.log("Proxy Endpoint:", endpoint);
      setOutput("Sending mint request...");

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
        throw new Error(`Mint request failed: ${errorText}`);
      }
      const data = await response.json();
      console.log("Mint Response:", data);
      let finalOutput = JSON.stringify(data, null, 2);

      let noticeString = "";
      let bytes32Value = "";
      let decodedInner = [];

      if (data.service_response && data.service_response[1]) {
        const secondResponse = data.service_response[1];
        const digestKey = Object.keys(secondResponse)[0];
        const noticeArray = secondResponse[digestKey][0][1];
        const fullBuffer = Buffer.from(noticeArray);
        console.log("Full Notice Buffer (hex):", fullBuffer.toString("hex"));
        finalOutput += "\n\nFull Notice Buffer (hex): " + fullBuffer.toString("hex");

        const iface = new ethers.utils.Interface(["function Notice(bytes data)"]);
        const decodedCall = iface.decodeFunctionData("Notice", "0x" + fullBuffer.toString("hex"));
        console.log("Decoded Notice Call:", decodedCall);
        const innerPayload = decodedCall.data;
        finalOutput += "\n\nInner Payload (hex): " + innerPayload;
        decodedInner = ethers.utils.defaultAbiCoder.decode(
          ["string", "bytes32"],
          innerPayload
        );
        console.log("Decoded Notice Payload:", decodedInner);
        finalOutput += "\n\nDecoded Notice Payload:";
        finalOutput += "\n  String: " + decodedInner[0];
        finalOutput += "\n  Bytes32: " + decodedInner[1].toString();
        noticeString = decodedInner[0];
        bytes32Value = decodedInner[1].toString();
      }

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

      const helia = await createHelia({
        start: false
      });
      console.log("Helia Node Initialized:", helia);
      finalOutput += "\n\nHelia Node Initialized: Peer ID: " + helia.libp2p.peerId.toString();

      let carFiles = [];
        if (Array.isArray(decodedPreimage)) {
          for (const [index, entry] of decodedPreimage.entries()) {
            const cidText = entry[0];
            const blockArray = entry[1];
            const blockBuffer = Buffer.from(blockArray);
            const cid = CID.parse(cidText);
            await helia.blockstore.put(cid, blockBuffer);
            console.log(`Stored block ${index} with CID:`, cid.toString());
            finalOutput += `\nStored block ${index} with CID: ${cid.toString()}`;
          }
        } else {
          console.warn("Decoded preimage is not an array");
          finalOutput += "\nDecoded preimage is not an array.";
        }
        finalOutput += "\n\nCAR Files Count: " + carFiles.length;

      if (noticeString) {
        const noticeCID = CID.parse(noticeString);
        const { writer, out } = await CarWriter.create([noticeCID]);
        const carBlob = await carWriterOutToBlob(out);
        await car(helia).export(noticeCID, writer);
        console.log(await carBlob);
        // await writer.put({ noticeCID, bytes: noticeBuffer });
        // await writer.close();
        // const chunks = [];
        // for await (const chunk of out) {
        //   chunks.push(chunk);
        // }
        console.log("Notice CID:", noticeCID);
        finalOutput += "\n\nNotice CAR Blob: " + (await carBlob);
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