import { BigDecimal, BigInt, Bytes, ethereum } from "@graphprotocol/graph-ts";
import {
  Account,
  AccountBalance,
  AccountBalanceSnapshot,
  AccountInventory,
  AccountInventorySnapshot,
  Nft,
  NftItem,
  Token,
  Transaction,
} from "../../generated/schema";
import { ZERO } from "../helpers/number";

export function getOrCreateAccount(accountAddress: Bytes): Account {
  let accountId = accountAddress.toHex();
  let existingAccount = Account.load(accountId);

  if (existingAccount != null) {
    return existingAccount as Account;
  }

  let newAccount = new Account(accountId);
  newAccount.address = accountAddress;

  return newAccount;
}

function getOrCreateAccountBalance(account: Account, token: Token): AccountBalance {
  let balanceId = account.id + "-" + token.id;
  let previousBalance = AccountBalance.load(balanceId);

  if (previousBalance != null) {
    return previousBalance as AccountBalance;
  }

  let newBalance = new AccountBalance(balanceId);
  newBalance.account = account.id;
  newBalance.asset = token.id;
  newBalance.token = token.id;
  newBalance.amount = ZERO.toBigDecimal();

  return newBalance;
}

export function increaseAccountBalance(account: Account, token: Token, amount: BigDecimal): AccountBalance {
  let balance = getOrCreateAccountBalance(account, token);
  balance.amount = balance.amount.plus(amount);

  return balance;
}

export function decreaseAccountBalance(account: Account, token: Token, amount: BigDecimal): AccountBalance {
  let balance = getOrCreateAccountBalance(account, token);
  balance.amount = balance.amount.minus(amount);

  return balance;
}

export function saveAccountBalanceSnapshot(balance: AccountBalance, eventId: string, event: ethereum.Event, tx: Transaction): void {
  let snapshot = new AccountBalanceSnapshot(balance.id + "-" + event.block.timestamp.toString());
  snapshot.account = balance.account;
  snapshot.token = balance.token;
  snapshot.amount = balance.amount;

  snapshot.block = event.block.number;
  snapshot.transaction = tx.id;
  snapshot.timestamp = event.block.timestamp;

  snapshot.event = eventId;

  snapshot.save();
}

type TokenFilter = (value: BigInt, index: i32, array: BigInt[]) => boolean;

function createTokenFilter(tokenId: BigInt): TokenFilter {
  return (value: BigInt, index: i32, array: BigInt[]) => {
    return tokenId.notEqual(value);
  };
}

export function getOrCreateAccountInventory(account: Account, token: Nft): AccountInventory {
  let balanceId = account.id + "-" + token.id;
  let previousInventory = AccountInventory.load(balanceId);

  if (previousInventory != null) {
    return previousInventory as AccountInventory;
  }

  let newInventory = new AccountInventory(balanceId);
  newInventory.account = account.id;
  newInventory.asset = token.id;
  newInventory.token = token.id;
  newInventory.tokenIds = [];

  return newInventory;
}

export function addTokenToAccountInventory(account: Account, token: Nft, item: NftItem): AccountInventory {
  let inventory = getOrCreateAccountInventory(account, token);
  let tokenIds = inventory.tokenIds;
  tokenIds.push(item.tokenId);
  inventory.tokenIds = tokenIds;
  return inventory;
}

export function removeTokenFromAccountInventory(account: Account, token: Nft, item: NftItem): AccountInventory {
  let inventory = getOrCreateAccountInventory(account, token);
  inventory.tokenIds = inventory.tokenIds.filter(createTokenFilter(item.tokenId));

  return inventory;
}

export function saveAccountInventorySnapshot(balance: AccountInventory, eventId: string, event: ethereum.Event, tx: Transaction): void {
  let snapshot = new AccountInventorySnapshot(balance.id + "-" + event.block.timestamp.toString());
  snapshot.account = balance.account;
  snapshot.token = balance.token;
  snapshot.tokenIds = balance.tokenIds;

  snapshot.block = event.block.number;
  snapshot.transaction = tx.id;
  snapshot.timestamp = event.block.timestamp;

  snapshot.event = eventId;

  snapshot.save();
}
