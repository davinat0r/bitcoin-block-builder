// use bitcoin_block_builder::feerate::{choose_txs_to_inlcude_in_block, read_csv_mempool_noscaling};
// 
// fn main() {
//     let mut mempool = read_csv_mempool_noscaling().unwrap();
//     choose_txs_to_inlcude_in_block(&mut mempool);
// }

use bitcoin_block_builder::knapsack::{choose_txs_to_inlcude_in_block, read_csv_mempool};

fn main() {
    let mut mempool = read_csv_mempool().unwrap();
    choose_txs_to_inlcude_in_block(&mut mempool);
}
