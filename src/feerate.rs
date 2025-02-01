use std::{error::Error, fs::File};
use std::io::Write;
use csv::StringRecord;
use serde::Deserialize;

pub const MAX_WEIGHT: f64 = 4_000_000.0;

#[derive(Debug, Deserialize, Default, Clone)]
pub struct MempoolIstance {
    pub txid: String,
    pub fee: f64,
    pub weight: f64,
    pub parent_txids: Vec<String>,
    #[serde(default)]
    pub whole_chain_indexes: Vec<usize>,
    #[serde(default)]
    pub already_included: bool,
    #[serde(default)]
    pub chain_fee: f64,
    #[serde(default)]
    pub chain_weight: f64,
    #[serde(default)]
    pub fee_rate: f64,
}

pub fn read_csv_mempool_noscaling() -> Result<Vec<MempoolIstance>, Box<dyn Error>> {
    let csv_file = File::open("./mempool.csv")?;
    let mut rdr = csv::Reader::from_reader(&csv_file);
    rdr.set_headers(StringRecord::from(vec!["txid", "fee", "weight", "parent_txids"]));

    let mut mempool = Vec::new();
    for result in rdr.deserialize() {
        let mut record: MempoolIstance = result?;
        let parent_txids: Vec<String> = record.parent_txids[0].as_str().split(";").map(|s| s.to_string()).collect();
        record.fee = record.fee;
        record.weight = record.weight;
        record.parent_txids = parent_txids;
        mempool.push(record);
    }
    Ok(mempool)
}

// Starting from a tx reconstructs the whole ancestors and returns the sum of (fees, wights)
// if at `tx_index` the mempool tx has no parents just return the (fee, weight) tuple
fn get_all_parents(mempool: &mut Vec<MempoolIstance>, root_index: usize, tx_index: usize) {
    let child_tx = mempool[tx_index].clone();

    if child_tx.parent_txids.len() == 1 && child_tx.parent_txids[0].cmp(&"".to_string()).is_eq() {
        return 
    }

    // for each construct the ancestor's chain. So for each parent loop "back" until there are no more parents found
    for parent in &child_tx.parent_txids {
        let has_parent: &Option<usize> = &mempool.iter().position(|m: &MempoolIstance| m.txid.cmp(parent).is_eq());
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

fn check_chain_not_included(mempool: &Vec<MempoolIstance>, index: usize) -> bool {
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

pub fn choose_txs_to_inlcude_in_block(mempool: &mut Vec<MempoolIstance>) {
    for idx in 0..mempool.len() {
        get_all_parents(mempool, idx, idx);
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
        mempool[index].fee_rate = fee_sum / weight_sum;
    }

    let mut mempool_sorted = mempool.clone();

    // order by fee_rate in desceding order
    mempool_sorted.sort_by(|a, b| {
        b.fee_rate.partial_cmp(&a.fee_rate).unwrap()
    });

    let mut file = File::create("./block.txt").unwrap();
    let mut w = MAX_WEIGHT;
    let mut total_fee_in_block = 0.0;
    let mut sorted_idx = 0;
    while sorted_idx < mempool_sorted.len() {
        // map sorted idx in original mempool idx (chain indexes refer to the original mempool -> after sorting we lose the indexes meaning)
        let curr_txid = &mempool_sorted[sorted_idx].txid;
        let idx = mempool.iter().position(|m| m.txid.cmp(curr_txid).is_eq()).unwrap();
        if check_chain_not_included(&mempool, idx) {
            if w-mempool[idx].chain_weight >= 0.0 {
                for p_idx in mempool[idx].whole_chain_indexes.clone() {
                    mempool[p_idx].already_included = true;
                    writeln!(file, "{}", mempool[p_idx].txid.clone()).unwrap();
                }
                mempool[idx].already_included = true;
                writeln!(file, "{}", mempool[idx].txid.clone()).unwrap();
                w-=mempool[idx].chain_weight;
                total_fee_in_block+=mempool[idx].chain_fee;
            }
        }
        sorted_idx+=1;
    }
    println!("Remaining weight {}", w);
    println!("Total fee in block {}", total_fee_in_block);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a() {
        let mut mempool = read_csv_mempool_noscaling().unwrap();
        choose_txs_to_inlcude_in_block(&mut mempool);
    }
}