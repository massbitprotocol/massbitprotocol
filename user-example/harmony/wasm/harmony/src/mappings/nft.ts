import { Bytes, BigInt } from "@graphprotocol/graph-ts";
import { Transfer } from "../../generated/templates/NFT/ERC721";
import { Nft, NftMintEvent, NftBurnEvent, NftTransferEvent, NftItem, Transaction } from "../../generated/schema";
import { ONE } from "../helpers/number";
import {
  addTokenToAccountInventory,
  getOrCreateAccount,
  getOrCreateAccountInventory,
  removeTokenFromAccountInventory,
  saveAccountInventorySnapshot,
} from "./account";
import { loadNFT, loadNFTItem, loadTransaction } from "../store";

const GENESIS_ADDRESS = "0x0000000000000000000000000000000000000000";

export function handleTransfer(event: Transfer): void {
  let token = loadNFT(event.address);

  if (token != null) {
    let tokenId = event.params.tokenId;

    let item = NftItem.load(event.address.toHex() + "-" + tokenId.toString());

    let isBurn = event.params.to.toHex() == GENESIS_ADDRESS;
    let isMint = event.params.from.toHex() == GENESIS_ADDRESS || item == null;

    item = loadNFTItem(event.address, tokenId) as NftItem;

    let isTransfer = !isBurn && !isMint;

    let tx = loadTransaction(event);

    // Update token event logs
    let eventEntityId: string;

    if (isBurn) {
      let eventEntity = handleBurnEvent(token, item as NftItem, event.params.from, event, tx);

      eventEntityId = eventEntity.id;
    } else if (isMint) {
      let eventEntity = handleMintEvent(token, item as NftItem, event.params.to, event, tx);

      eventEntityId = eventEntity.id;
    } else if (isTransfer) {
      let eventEntity = handleTransferEvent(token, item as NftItem, event.params.from, event.params.to, event, tx);

      eventEntityId = eventEntity.id;
    }

    // Updates balances of accounts
    if (isTransfer || isBurn) {
      let sourceAccount = getOrCreateAccount(event.params.from);

      let accountInventory = removeTokenFromAccountInventory(sourceAccount, token as Nft, item as NftItem);

      accountInventory.block = event.block.number;
      accountInventory.modified = event.block.timestamp;
      accountInventory.transaction = tx.id;

      sourceAccount.save();
      accountInventory.save();
      saveAccountInventorySnapshot(accountInventory, eventEntityId, event, tx);
    }

    if (isTransfer || isMint) {
      let destinationAccount = getOrCreateAccount(event.params.to);

      let accountInventory = addTokenToAccountInventory(destinationAccount, token as Nft, item as NftItem);
      accountInventory.block = event.block.number;
      accountInventory.modified = event.block.timestamp;
      accountInventory.transaction = tx.id;

      destinationAccount.save();
      accountInventory.save();
      saveAccountInventorySnapshot(accountInventory, eventEntityId, event, tx);
    }
  }
}

type TokenFilter = (value: BigInt, index: i32, array: BigInt[]) => boolean;

function createTokenFilter(tokenId: BigInt): TokenFilter {
  return (value: BigInt, index: i32, array: BigInt[]) => {
    return tokenId.notEqual(value);
  };
}

function handleBurnEvent(token: Nft | null, item: NftItem, burner: Bytes, event: Transfer, tx: Transaction): NftBurnEvent {
  let burnEvent = new NftBurnEvent(event.transaction.hash.toHex() + "-" + event.logIndex.toString());
  burnEvent.type = "BURN";
  burnEvent.asset = event.address.toHex();
  burnEvent.token = event.address.toHex();
  burnEvent.item = item.id;
  burnEvent.tokenId = item.tokenId;
  burnEvent.sender = event.transaction.from;
  burnEvent.burner = burner;

  burnEvent.block = event.block.number;
  burnEvent.timestamp = event.block.timestamp;
  burnEvent.transaction = tx.id;

  burnEvent.save();

  if (token != null) {
    token.eventCount = token.eventCount.plus(ONE);
    token.burnEventCount = token.burnEventCount.plus(ONE);
    token.totalSupply = token.totalSupply.minus(ONE);
    token.totalBurned = token.totalBurned.plus(ONE);

    token.tokenIds = token.tokenIds.filter(createTokenFilter(event.params.tokenId));

    token.save();

    item.eventCount = item.eventCount.plus(ONE);
    item.owner = null;
    item.ownerInventory = null;
    item.burner = burner;
    item.burn = burnEvent.id;
    item.save();
  }

  return burnEvent;
}

function handleMintEvent(token: Nft | null, item: NftItem, destination: Bytes, event: Transfer, tx: Transaction): NftMintEvent {
  let mintEvent = new NftMintEvent(event.transaction.hash.toHex() + "-" + event.logIndex.toString());
  mintEvent.type = "MINT";
  mintEvent.asset = event.address.toHex();
  mintEvent.token = event.address.toHex();
  mintEvent.item = item.id;
  mintEvent.tokenId = item.tokenId;
  mintEvent.sender = event.transaction.from;
  mintEvent.destination = destination;
  mintEvent.minter = event.transaction.from;

  mintEvent.block = event.block.number;
  mintEvent.timestamp = event.block.timestamp;
  mintEvent.transaction = tx.id;

  mintEvent.save();

  if (token != null) {
    token.eventCount = token.eventCount.plus(ONE);
    token.mintEventCount = token.mintEventCount.plus(ONE);
    token.totalSupply = token.totalSupply.plus(ONE);
    token.totalMinted = token.totalMinted.plus(ONE);

    let tokenIds = token.tokenIds;
    tokenIds.push(item.tokenId);
    token.tokenIds = tokenIds;
    token.save();

    let destinationAccount = getOrCreateAccount(destination);
    let destinationAccountInventory = getOrCreateAccountInventory(destinationAccount, token as Nft);

    item.eventCount = item.eventCount.plus(ONE);
    item.minter = event.transaction.from;
    item.mint = mintEvent.id;
    item.owner = destinationAccount.id;
    item.ownerInventory = destinationAccountInventory.id;
    item.save();
  }

  return mintEvent;
}

function handleTransferEvent(
  token: Nft | null,
  item: NftItem,
  source: Bytes,
  destination: Bytes,
  event: Transfer,
  tx: Transaction
): NftTransferEvent {
  let transferEvent = new NftTransferEvent(event.transaction.hash.toHex() + "-" + event.logIndex.toString());
  transferEvent.type = "TRANSFER";
  transferEvent.asset = event.address.toHex();
  transferEvent.token = event.address.toHex();
  transferEvent.item = item.id;
  transferEvent.tokenId = item.tokenId;
  transferEvent.sender = source;
  transferEvent.source = source;
  transferEvent.destination = destination;

  transferEvent.block = event.block.number;
  transferEvent.timestamp = event.block.timestamp;
  transferEvent.transaction = tx.id;

  transferEvent.save();

  if (token != null) {
    token.eventCount = token.eventCount.plus(ONE);
    token.transferEventCount = token.transferEventCount.plus(ONE);

    token.save();

    let destinationAccount = getOrCreateAccount(destination);
    let destinationAccountInventory = getOrCreateAccountInventory(destinationAccount, token as Nft);

    item.eventCount = item.eventCount.plus(ONE);
    item.transferEventCount = token.transferEventCount.plus(ONE);
    item.owner = destinationAccount.id;
    item.ownerInventory = destinationAccountInventory.id;
    item.save();
  }

  return transferEvent;
}
