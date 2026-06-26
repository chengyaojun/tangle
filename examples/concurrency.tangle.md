# ConcurrencyDemo

Demonstrates Task, Channel, and Sync operations.

#### main

```@tangle
let mutex = Sync.mutex_new()
Sync.mutex_lock(mutex)
Sync.mutex_unlock(mutex)
let ch = Channel.new(10)
Channel.send(ch, "hello")
let msg = Channel.recv(ch)
return msg
```
