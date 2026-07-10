# AccountDemo

### Account
* `balance`: current balance (Int)

#### open
* `initial`: starting balance (Int)

```@tangle
return Account { balance: initial }
```

#### deposit
* `account`: account to update (Account)
* `amount`: amount to add (Int)

```@tangle
return account { balance: account.balance + amount }
```

#### main

```@tangle
let acc = Account.open(100)?
let acc2 = Account.deposit(acc, 50)?
return acc2
```
