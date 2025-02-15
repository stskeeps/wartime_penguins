import React, { useState } from "react";
import { ethers } from "ethers";
import { Buffer } from "buffer";
import { decode as cborDecode } from "cbor2";
import { createHelia } from "helia";
import { CID } from "multiformats/cid";
import { CarWriter } from "@ipld/car/writer";
import { car } from "@helia/car";
import { unixfs } from "@helia/unixfs";
import { keccak256 } from '@multiformats/sha3'
import { sha256 } from 'multiformats/hashes/sha2'

async function carWriterOutToBlob(carReaderIterable) {
  const parts = [];
  for await (const part of carReaderIterable) {
    parts.push(part);
  }
  return new Blob(parts, { type: "application/car" });
}

function App() {
  const [walletAddress, setWalletAddress] = useState("");
  const [seed, setSeed] = useState("");
  const [output, setOutput] = useState("");
  const [nftMetadata, setNftMetadata] = useState(null);
  const [nftImageUrl, setNftImageUrl] = useState("");
  const [nftCID, setNftCID] = useState("");
  const [carBlob, setCarBlob] = useState(null);
  const [helia, setHelia] = useState(null);

  const connectWallet = async () => {
    if (window.ethereum)
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
  };

  // Renamed from handleMint to handleComputeNFT
  const handleComputeNFT = async () => {
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

      const machineHash =
        "4f9e81934a1096047822d363dc5c5c05348fc2ed19cc54e0814eb37612d52018";
      const fixedAddress = "0xA44151489861Fe9e3055d95adC98FbD462B948e7";
      const endpoint = "http://localhost:3001/issue_task";
      console.log("Proxy Endpoint:", endpoint);
      setOutput("Compute nft request...");

      const response = await fetch(endpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          machineHash,
          fixedAddress,
          input: encodedInput,
        }),
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
        finalOutput +=
          "\n\nFull Notice Buffer (hex): " + fullBuffer.toString("hex");

        const iface = new ethers.utils.Interface([
          "function Notice(bytes data)",
        ]);
        const decodedCall = iface.decodeFunctionData(
          "Notice",
          "0x" + fullBuffer.toString("hex")
        );
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
        setNftCID(noticeString);
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
      finalOutput +=
        "\n\nPreimage Buffer (hex): " + preimageBuffer.toString("hex");

      const decodedPreimage = cborDecode(preimageBuffer);
      console.log("Decoded Preimage (CBOR):", decodedPreimage);
      finalOutput +=
        "\n\nDecoded Preimage (CBOR): " +
        JSON.stringify(decodedPreimage, null, 2);

      const helia = await createHelia({ start: false,
        getHasher: (initialHashers, loadHasher) => {
          return async (code) => {
            if (code === keccak256.code) {
              return keccak256;
            } else if (code === sha256.code) {
              return sha256;
            } else {
              throw new Error("Unsupported hash code: " + code);
            }
          }
        }
      });
      setHelia(helia);


      console.log("Helia Node Initialized:", helia);
      finalOutput +=
        "\n\nHelia Node Initialized: Peer ID: " +
        helia.libp2p.peerId.toString();

      let carFiles = [];
      if (Array.isArray(decodedPreimage)) {
        for (const [index, entry] of decodedPreimage.entries()) {
          const cidText = entry[0];
          const blockArray = entry[1];
          const blockHash = Buffer.from(blockArray);
          const cid = CID.parse(cidText);
          const solverEndpoint = `http://localhost:3001/get_preimage/2/${blockHash.toString(
            "hex"
          )}`;

          const preimageResponse = await fetch(solverEndpoint);
          if (!preimageResponse.ok) {
            const errText = await preimageResponse.text();
            throw new Error("Solver preimage request failed: " + errText);
          }
          const blockBuffer = await preimageResponse.arrayBuffer();
          await helia.blockstore.put(cid, new Uint8Array(blockBuffer));
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
        console.log("Notice CID:", noticeCID);
        console.log("Notice CID string:", noticeCID.toString());
        const fs = unixfs(helia);
        for await (const entry of fs.ls(noticeCID)) {
          console.info(entry);
        }
        const { writer, out } = await CarWriter.create([noticeCID]);
        const carBlob = carWriterOutToBlob(out);
        await car(helia).export(noticeCID, writer);
        console.log(await carBlob);
        // await writer.put({ noticeCID, bytes: noticeBuffer });
        // await writer.close();
        // const chunks = [];
        // for await (const chunk of out) {
        //   chunks.push(chunk);
        // }

        finalOutput += "\n\nNotice CAR Blob: " + carBlob;
      }

      setOutput(finalOutput);
    } catch (error) {
      console.error(error);
      setOutput("Error: " + error.message);
    }
  };

  const handleViewNFT = async () => {
    if (!nftCID) return alert("No NFT CID available.");
    try {
      const fs = unixfs(helia);
      const cid = CID.parse(
        nftCID.startsWith("ipfs://") ? nftCID.slice(7) : nftCID
      );
      const chunks = [];
      for await (const chunk of fs.cat(cid, { path: "metadata.json" })) {
        chunks.push(chunk);
      }
      const metadataBuffer = Buffer.concat(chunks);
      const metadataText = metadataBuffer.toString("utf8");
      const metadata = JSON.parse(metadataText);
      let imageCID = metadata.image;
      if (imageCID.startsWith("ipfs://")) imageCID = imageCID.slice(7);
      const imgCID = CID.parse(imageCID);
      const imageChunks = [];
      for await (const chunk of fs.cat(imgCID)) {
        imageChunks.push(chunk);
      }
      const imageBuffer = Buffer.concat(imageChunks);
      const imageBlob = new Blob([imageBuffer], { type: "image/gif" });
      const localImageUrl = URL.createObjectURL(imageBlob);
      setNftMetadata(metadata);
      setNftImageUrl(localImageUrl);
    } catch (error) {
      console.error(error);
      alert("Error viewing NFT: " + error.message);
    }
  };

  const handleMintNFT = async () => {
    if (!walletAddress) {
      alert("Connect your wallet first!");
      return;
    }
    if (!seed) {
      alert("Please enter a seed value!");
      return;
    }
    try {
      const provider = new ethers.providers.Web3Provider(window.ethereum);
      const signer = provider.getSigner();

      const contractAddress = "0x680bBA4E54f62caafC906B9382C150603a7EF226";

      const myContractABI = [
        "function requestmint(uint256 seed) external",
        "function mintsInProgress(bytes32) public view returns (address)"
      ];

      const contract = new ethers.Contract(contractAddress, myContractABI, signer);

      const seedValue = ethers.BigNumber.from(seed);
      let gasLimit;
      try {
        gasLimit = await contract.estimateGas.requestmint(seedValue);
        console.log("Estimated gas:", gasLimit.toString());
      } catch (gasErr) {
        console.warn("Gas estimation failed, using fallback gas limit", gasErr);
        gasLimit = ethers.BigNumber.from("2000000");
      }

      const tx = await contract.requestmint(seedValue, { gasLimit });
      setOutput("RequestMint sent, tx hash: " + tx.hash);
      console.log("Transaction sent:", tx.hash);
      const receipt = await tx.wait();
      if (receipt.status === 0) {
        throw new Error("Transaction reverted");
      }
      setOutput(`RequestMint confirmed in block ${receipt.blockNumber}`);
      console.log("Transaction receipt:", receipt);
      alert("Mint request sent. The NFT will be minted once the coprocessor calls handleNotice.");
    } catch (error) {
      console.error("Error requesting mint:", error);
      let errMsg = error.message;
      if (error.data) {
        errMsg += " | " + JSON.stringify(error.data);
      }
      setOutput("Error requesting mint: " + errMsg);
    }
  };

  return (
    <div className="App">
      <div className="container">
        <img
          src="/penguin.png"
          alt="Header"
          style={{ width: "100%", marginBottom: "20px" }}
        />
        <h1>Mint Your NFT</h1>
        <button onClick={connectWallet}>Connect Wallet</button>
        <input
          type="number"
          placeholder="Enter a seed (uint256)"
          value={seed}
          onChange={(e) => setSeed(e.target.value)}
        />
        <button onClick={handleComputeNFT}>Compute NFT</button>
        <button onClick={handleViewNFT}>View NFT</button>
        {carBlob && (
          <div>
            <a href={URL.createObjectURL(carBlob)} download="notice.car">
              Download CAR File
            </a>
          </div>
        )}
        <button onClick={handleMintNFT}>Mint NFT</button>
        {nftMetadata && (
          <div>
            <h2>NFT Metadata</h2>
            <pre>{JSON.stringify(nftMetadata, null, 2)}</pre>
            {nftImageUrl && (
              <img src={nftImageUrl} alt="NFT" style={{ maxWidth: "300px" }} />
            )}
          </div>
        )}
        <pre className="output">{output}</pre>
      </div>
    </div>
  );
}

export default App;
