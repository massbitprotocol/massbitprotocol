import { Address, log, BigInt, ethereum } from "@graphprotocol/graph-ts";
import { Nft, NftItem, Token, Transaction } from "../generated/schema";
import { ERC20 } from "../generated/templates/StandardToken/ERC20";
import { ERC721 } from "../generated/templates/NFT/ERC721";
import { toDecimal, ZERO } from "./helpers/number";

export function loadToken(address: Address): Token {
  let token = Token.load(address.toHex());

  if (token == null) {
    let erc20 = ERC20.bind(address);
    let initialSupply = erc20.try_totalSupply();
    let name = erc20.try_name();
    let symbol = erc20.try_symbol();
    let decimals = erc20.try_decimals();

    token = new Token(address.toHex());
    token.type = "TOKEN";
    token.address = address;
    token.name = name.reverted ? "unknow" : name.value;
    token.symbol = symbol.reverted ? "unknow" : symbol.value;
    token.decimals = decimals.reverted ? 0 : decimals.value;

    token.eventCount = ZERO;
    token.burnEventCount = ZERO;
    token.mintEventCount = ZERO;
    token.transferEventCount = ZERO;

    token.totalSupply = initialSupply.reverted ? ZERO.toBigDecimal() : toDecimal(initialSupply.value, token.decimals);
    token.totalBurned = ZERO.toBigDecimal();
    token.totalMinted = ZERO.toBigDecimal();
    token.totalTransferred = ZERO.toBigDecimal();

    log.debug("Adding token to registry, name: {}, symbol: {}, address: {}, decimals: {}", [
      token.name,
      token.symbol,
      token.id,
      BigInt.fromI32(decimals.value).toString(),
    ]);

    token.save();
  }

  return token as Token;
}

export function loadNFT(address: Address): Nft {
  let token = Nft.load(address.toHex());

  if (token == null) {
    let erc721 = ERC721.bind(address);
    let name = erc721.try_name();
    let symbol = erc721.try_symbol();

    token = new Nft(address.toHex());
    token.type = "NFT";
    token.address = address;
    token.name = name.reverted ? "unknow" : name.value;
    token.symbol = symbol.reverted ? "unknow" : symbol.value;

    token.eventCount = ZERO;
    token.burnEventCount = ZERO;
    token.mintEventCount = ZERO;
    token.transferEventCount = ZERO;

    token.totalSupply = ZERO;
    token.totalBurned = ZERO;
    token.totalMinted = ZERO;

    token.tokenIds = [];

    log.debug("Adding nft to registry, name: {}, symbol: {}, address: {}", [token.name, token.symbol, token.id]);

    token.save();
  }

  return token as Nft;
}

export function loadNFTItem(address: Address, tokenId: BigInt): NftItem {
  let item = NftItem.load(address.toHex() + "-" + tokenId.toString());

  if (item == null) {
    let erc721 = ERC721.bind(address);
    let uri = erc721.try_tokenURI(tokenId);

    item = new NftItem(address.toHex() + "-" + tokenId.toString());
    item.token = address.toHex();
    item.tokenId = tokenId;
    item.tokenUri = uri.reverted ? "" : uri.value;
    item.eventCount = ZERO;
    item.transferEventCount = ZERO;

    item.save();
  }

  return item as NftItem;
}

export function loadTransaction(event: ethereum.Event): Transaction {
  let tx = Transaction.load(event.transaction.hash.toHex());

  if (tx == null) {
    tx = new Transaction(event.transaction.hash.toHex());
    tx.blockNumber = event.block.number;
    tx.blockHash = event.block.hash;
    tx.hash = event.transaction.hash;
    tx.index = event.transaction.index;
    tx.from = event.transaction.from;
    tx.to = event.transaction.to;
    tx.value = event.transaction.value;
    tx.gasUsed = event.transaction.gasUsed;
    tx.gasPrice = event.transaction.gasPrice;
    tx.input = event.transaction.input;
    tx.timestamp = event.block.timestamp;
    tx.save();
  }

  return tx as Transaction;
}
