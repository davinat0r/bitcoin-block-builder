use std::cmp::max;
use std::collections::{HashMap, HashSet};
use std::{error::Error, fs::File, vec};
use std::io::Write;
use csv::StringRecord;
use serde::Deserialize;

use crate::feerate::read_csv_mempool_noscaling;

// the max weight is 4,000,000 / 100
const MAX_WEIGHT: usize = 40000;

#[derive(Debug, Deserialize, Default, Clone)]
pub struct MempoolCsvRow {
    pub txid: String,
    pub fee: u64,
    pub weight: u64,
    pub parent_txids: Vec<String>,
    #[serde(default)]
    pub whole_chain_indexes: Vec<usize>,
    #[serde(default)]
    pub already_included: bool,
    #[serde(default)]
    pub chain_fee: u64,
    #[serde(default)]
    pub chain_weight: u64,
}


pub fn read_csv_mempool() -> Result<Vec<MempoolCsvRow>, Box<dyn Error>> {
    let csv_file = File::open("./mempool.csv")?;
    let mut rdr = csv::Reader::from_reader(&csv_file);
    rdr.set_headers(StringRecord::from(vec!["txid", "fee", "weight", "parent_txids"]));

    let mut mempool = Vec::new();
    for result in rdr.deserialize() {
        let mut record: MempoolCsvRow = result?;
        let parent_txids: Vec<String> = record.parent_txids[0].as_str().split(";").map(|s| s.to_string()).collect();
        record.fee = (record.fee as f64 / 100.0).ceil() as u64;
        record.weight = (record.weight as f64 / 100.0).ceil() as u64;
        record.parent_txids = parent_txids;
        mempool.push(record);
    }
    Ok(mempool)
}

fn check_chain_not_included(mempool: &Vec<MempoolCsvRow>, index: usize) -> bool {
    if mempool[index].already_included {
        return false
    }
    let curr_tx_chain_idxs = mempool[index].whole_chain_indexes.clone();
    for idx in curr_tx_chain_idxs {
        if mempool[idx].already_included {
            return false
        }
    };
    true
}

// Starting from a tx reconstructs the whole ancestors and returns the sum of (fees, wights)
// if at `tx_index` the mempool tx has no parents just return the (fee, weight) tuple
fn get_all_parents(mempool: &mut Vec<MempoolCsvRow>, root_index: usize, tx_index: usize) {
    let child_tx = mempool[tx_index].clone();

    if child_tx.parent_txids.len() == 1 && child_tx.parent_txids[0].cmp(&"".to_string()).is_eq() {
        return 
    }

    // for each construct the ancestor's chain. So for each parent loop "back" until there are no more parents found
    for parent in &child_tx.parent_txids {
        let has_parent: &Option<usize> = &mempool.iter().position(|m: &MempoolCsvRow| m.txid.cmp(parent).is_eq());
        match has_parent {
            Some(p_idx) => {
                get_all_parents(mempool, root_index, *p_idx);
                if !mempool[root_index].whole_chain_indexes.contains(p_idx) { // avoid duplicates
                    mempool[root_index].whole_chain_indexes.push(*p_idx);
                }
            },
            None => break,
        }
    }
}

pub fn get_all_parents2(mempool: &HashMap<String, MempoolCsvRow>, current_txid: &String) -> HashSet<String> {
    let mut txids = HashSet::new();
    let v = mempool.get(current_txid).unwrap();
    if v.parent_txids.len() == 1 && v.parent_txids[0].cmp(&"".to_string()).is_eq() {
        return txids;
    }

    for p in &v.parent_txids {
        txids.insert(p.clone());
        let local_parents = get_all_parents2(&mempool, p);
        if local_parents.len() > 0 {
            txids.extend(local_parents);
        }
    }

    txids
}

pub fn choose_txs_to_inlcude_in_block(mempool: &mut Vec<MempoolCsvRow>){
    let mut knapsack = vec![vec![0u64; MAX_WEIGHT + 1]; mempool.len() + 1]; 

    // get all parent of the i-th tx and calculate the sum of fees and weight
    // this way in the matrix I will always include all the parents of the i-th tx.
    // When I will process the parent in a next iteration I will do the same, so I will
    // also include the case the I do not include the whole chain.
    for txid in 0..mempool.len() {
        get_all_parents(mempool, txid, txid);
    }
    for (index, row) in mempool.clone().iter().enumerate() {
        let mut fee_sum = row.fee;
        let mut weight_sum = row.weight;
        for p_idx in &row.whole_chain_indexes {
            fee_sum += mempool[*p_idx].fee;
            weight_sum += mempool[*p_idx].weight;
        }
        mempool[index].chain_fee = fee_sum;
        mempool[index].chain_weight = weight_sum;
    }

    for i in 1..knapsack.len() {
        for w in 1..knapsack[0].len() {
            if mempool[i-1].chain_weight <= w  as u64 {
                let include_item = mempool[i-1].chain_fee + knapsack[i-1][w-(mempool[i-1].chain_weight as usize)];
                let exclude_item = knapsack[i-1][w];
                knapsack[i][w] = max(include_item, exclude_item)
            } else {
                knapsack[i][w] = knapsack[i-1][w]
            }
        }
    }
    
    let mut file = File::create("./block.txt").unwrap();
    let mut w = MAX_WEIGHT;
    let mut i = knapsack.len() - 1;
    let mut j = knapsack[0].len() - 1;
    let mut total_fee = 0.0;
    let original_mempool = read_csv_mempool_noscaling().unwrap();
    while i > 0 && j > 0 && w > 0 {
        if knapsack[i][j] != knapsack[i-1][j] {
            if check_chain_not_included(mempool, i - 1) {
                for p_idx in mempool[i-1].whole_chain_indexes.clone() {
                    mempool[p_idx].already_included = true;
                    total_fee += original_mempool[p_idx].fee;
                    writeln!(file, "{}", mempool[p_idx].txid).unwrap();
                }
                writeln!(file, "{}", mempool[i-1].txid).unwrap();

                // mark current as included
                mempool[i-1].already_included = true;

                w-=mempool[i-1].chain_weight as usize;
                j-=mempool[i-1].chain_weight as usize;
                total_fee += original_mempool[i-1].fee;
            }
        }
        i-=1;
    }
    println!("Remaining weight {}", w);
    println!("Total fee {}", total_fee);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_parents() {
        let mut mempool = read_csv_mempool().unwrap();
        for idx in 0..mempool.len() {
            get_all_parents(&mut mempool, idx, idx);
        }
        for (index, row) in mempool.clone().iter().enumerate() {
            let mut fee_sum = row.fee;
            let mut weight_sum = row.weight;
            for p_idx in &row.whole_chain_indexes {
                fee_sum += mempool[*p_idx].fee;
                weight_sum += mempool[*p_idx].weight;
            }
            mempool[index].chain_fee = fee_sum;
            mempool[index].chain_weight = weight_sum;
        }
        let mut mempool_file = File::create("./mempool_modified.txt").unwrap();
        writeln!(mempool_file, "{:#?}", mempool).unwrap();
    }
}