# Block Builder 

Bitcoin miners construct blocks by selecting a set of transactions from their
mempool. Each transaction in the mempool:

- includes a _fee_ which is collected by the miner if that transaction is
  included in a block
- has a _weight_, which indicates the size of the transaction
- may have one or more _parent transactions_ which are also in the mempool

The miner selects an ordered list of transactions which have a combined weight
below the maximum block weight. Transactions with parent transactions in the
mempool may be included in the list, but only if _all_ of their parents appear
_before them_ in the list.

Naturally, the miner would like to include the transactions that maximize the
total fee.

Given a file [`mempool.csv`](mempool.csv),
with the format:

`<txid>,<fee>,<weight>,<parent_txids>`

- `txid` is the transaction identifier
- `fee` is the transaction fee
- `weight` is the transaction weight
- `parent_txids` is a list of the txids of the transaction’s **immediate**
  parent transactions in the mempool. Ancestors that are not immediate parents
  (eg parents of parents) and parent transactions that are already in the chain
  are not included in this list.
  It is of the form: `<txid1>;<txid2>;...`

The block builder outputs txids, separated by newlines, which
make a valid block, maximizing the fee to the miner. Transactions **MUST**
appear in order. No transaction should appear unless its parents are
included, no transaction should appear before its parents, and no
transaction may appear more than once.

## Input file

Here are two lines of the `mempool.csv` file:

```
2e3da8fbc1eaca8ed9b7c2db9e6545d8ccac3c67deadee95db050e41c1eedfc0,452,1620,
```

This is a transaction with txid `2e3da8...`, fees of 452 satoshis, weight of
1620, and no parent transactions in the mempool.

```
9d317fb308fd5451fd0ec612165638cb9e37bd8aa8918dff99a48fe05224276f,350,1400,288ea91bb52d8cb28289f4db0d857356622e39e78f33f26bf6df2bbdd3810fad;b5b993bda3c23bdefe4a1cf75b1f7cbdfe43058f2e4e7e25898f449375bb685c;c1ae3a82e52066b670e43116e7bfbcb6fa0abe16088f920060fa41e09715db7d
```

This is a transaction with txid `9d317f...`, fees of 350 satoshis, weight of
1400 and three parent transactions in the mempool with txids `288ea9....`,
`b5b993...` and `c1ae3a...`
