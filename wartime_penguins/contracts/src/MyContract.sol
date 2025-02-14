// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC721/ERC721.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "../lib/coprocessor-base-contract/src/CoprocessorAdapter.sol";

contract MyContract is ERC721, CoprocessorAdapter, Ownable {
    mapping(uint256 => string) private _tokenURIs;
    mapping(bytes32 => address) public mintsInProgress;

    constructor(address _taskIssuerAddress, bytes32 _machineHash)
        ERC721("Wartime Penguins", "WARPENGU")
        CoprocessorAdapter(_taskIssuerAddress, _machineHash)
        Ownable(msg.sender)
    {}

    function requestmint(uint256 seed) external {
        bytes memory encoded = abi.encode(msg.sender, seed);
        bytes32 mintKey = keccak256(encoded);
        mintsInProgress[mintKey] = msg.sender;
        callCoprocessorBytes(encoded);
    }

    function handleNotice(bytes32 payloadHash, bytes memory notice) internal override {
        (string memory uri, ) = abi.decode(
            notice,
            (string, bytes32)
        );

        address recipient = mintsInProgress[payloadHash];
        require(recipient != address(0), "No mint in progress for this hash");

        uint256 tokenId = uint256(payloadHash);

        delete mintsInProgress[payloadHash];
        _safeMint(recipient, tokenId);
        _tokenURIs[tokenId] = string.concat(string.concat("ipfs://", uri), "/metadata.json");
    }

    function callCoprocessorBytes(bytes memory input) internal {
        bytes32 inputHash = keccak256(input);
        computationSent[inputHash] = true;
        taskIssuer.issueTask(machineHash, input, address(this));
    }

    function tokenURI(uint256 tokenId) public view virtual override returns (string memory) {
        return _tokenURIs[tokenId];
    }

}