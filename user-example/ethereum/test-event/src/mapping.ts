import { Transfer } from '../generated/StandardToken/StandardToken'
import { StandardToken } from '../generated/schema'

export function handleTransfer(event: Transfer): void {
  let gravatar = new StandardToken(event.address.toHex())

  gravatar.from = event.params.from.toHex()
  gravatar.to = event.params.to.toHex()
  gravatar.value = event.params.value
  gravatar.save()
}