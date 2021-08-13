import { MasterChefModel } from '../generated/schema'

import { ethereum } from '@graphprotocol/graph-ts'
export function handleBlock(block: ethereum.Block): void {
  let id = block.hash.toHex()
  let entity = new MasterChefModel(id)
  entity.timestamp = block.timestamp;
  entity.save()
}