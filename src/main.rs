use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufReader, BufWriter},
    process::Command,
    slice::Iter,
};

fn main() {
    let snapshot: Snapshot = match File::open(".sol_gas.log") {
        Ok(file) => serde_json::from_reader(BufReader::new(file)).unwrap(),
        Err(_) => Snapshot::default(),
    };

    let binding = forge_gas();
    let gas_report = binding.lines();
    let gas_table = gas_report
        .filter(|line| line.contains('|'))
        .map(|line| {
            let line = line.trim_matches('|');
            let split = line.split('|');
            split.map(|element| element.trim()).collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let gas_table = parse_gas_table(&gas_table);

    println!("{}", snapshot.get_diff(&gas_table));

    let snapshot = Snapshot::from(&gas_table);

    let file = File::options()
        .write(true)
        .truncate(true)
        .create(true)
        .open(".sol_gas.log")
        .unwrap();
    let writter = BufWriter::new(file);
    serde_json::to_writer_pretty(writter, &snapshot).unwrap();
}

pub fn forge_gas() -> String {
    match Command::new("forge")
        .args(["test", "--gas-report"])
        .output()
    {
        Ok(output) => match output.status.code() {
            Some(0) => match String::from_utf8(output.stdout) {
                Ok(stdout) => stdout,
                Err(err) => {
                    unimplemented!("{}", err);
                }
            },
            _ => unimplemented!("{:?}", output.status.code()),
        },
        Err(err) => {
            unimplemented!("{}", err);
        }
    }
}

pub fn split_contracts<'a>(gas_table: &'a [Vec<&'a str>]) -> Vec<Iter<'_, std::vec::Vec<&str>>> {
    if gas_table.is_empty() {
        return vec![];
    }
    let gas_table_len = gas_table.len();

    let mut indexes = vec![];
    let mut contracts: Vec<Iter<'_, std::vec::Vec<&str>>> = vec![];

    gas_table.iter().enumerate().for_each(|(i, line)| {
        if get_empties(&line) == 5 {
            indexes.push(i);
        }
    });

    let indexes_len = indexes.len();

    if indexes_len == 1 {
        contracts.push(gas_table.iter());
    } else {
        contracts.push(gas_table[..*indexes.get(1).unwrap()].iter());
        for i in 1..indexes_len - 2 {
            contracts
                .push(gas_table[(*indexes.get(i).unwrap())..(*indexes.get(i + 1).unwrap())].iter());
        }
        contracts.push(gas_table[(*indexes.get(indexes_len - 1).unwrap())..].iter());
    }

    contracts
}

pub fn parse_gas_table<'a>(gas_table: &'a [Vec<&'a str>]) -> GasTable<'a> {
    let mut gas_iter = split_contracts(gas_table);

    let contracts = gas_iter
        .iter_mut()
        .map(|iter| {
            let contract_members: (&str, &str, &str);
            let deployment_cost: usize;
            let deployment_size: usize;
            let mut functions = vec![];

            if let Some(first) = iter.next() {
                let contract_line = first.first().unwrap();
                contract_members = {
                    let cl = contract_line.find(':').unwrap();
                    let sp = contract_line.find(' ').unwrap();
                    (
                        &contract_line[0..cl],
                        &contract_line[cl + 1..sp],
                        &contract_line[sp + 1..],
                    )
                };
            } else {
                todo!();
            }

            if let Some(line) = iter.next() {
                if !line.iter().all(|i| i.chars().all(|c| c == '-')) {
                    todo!();
                }
            } else {
                todo!();
            }

            if let Some(line) = iter.next() {
                if line.first() != Some(&"Deployment Cost")
                    || line.get(1) != Some(&"Deployment Size")
                {
                    todo!();
                }
            } else {
                todo!();
            }

            if let Some(cost_size) = iter.next() {
                deployment_cost = cost_size.first().unwrap().parse::<usize>().unwrap();
                deployment_size = cost_size.get(1).unwrap().parse::<usize>().unwrap();
            } else {
                todo!();
            }

            if let Some(cost_size) = iter.next() {
                // TODO:
            } else {
                todo!();
            }

            while let Some(function) = iter.next() {
                functions.push(Function {
                    name: function.first().unwrap(),
                    min: function.get(1).unwrap().parse::<usize>().unwrap(),
                    avg: function.get(2).unwrap().parse::<usize>().unwrap(),
                    median: function.get(3).unwrap().parse::<usize>().unwrap(),
                    max: function.get(4).unwrap().parse::<usize>().unwrap(),
                    calls: function.get(5).unwrap().parse::<usize>().unwrap(),
                });
            }

            Contract {
                file: contract_members.0,
                contract: contract_members.1,
                c_type: contract_members.2,
                deployment_cost,
                deployment_size,
                functions,
            }
        })
        .collect();

    GasTable { contracts }
}

pub fn get_empties(slice: &[&str]) -> usize {
    slice.iter().filter(|i| i.is_empty()).count()
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Snapshot {
    deployment_cost: usize,
    min_cost: usize,
    avg_cost: usize,
    // median_cost: usize,
    max_cost: usize,
}

impl Snapshot {
    pub fn get_diff(&self, gas_table: &GasTable<'_>) -> String {
        let mut result = String::new();

        if self.deployment_cost != gas_table.deployment_cost() {
            let diff = self.deployment_cost as isize - gas_table.deployment_cost() as isize;
            if diff.is_positive() {
                result.push_str(format!("Deployment gas cost reduced by {}\n", diff).as_str());
            } else {
                result.push_str(format!("Deployment gas cost increased by {}\n", -diff).as_str());
            }
        }

        if self.min_cost != gas_table.min_cost() {
            let diff = self.min_cost as isize - gas_table.min_cost() as isize;
            if diff.is_positive() {
                result.push_str(
                    format!("Minimum functions call gas cost reduced by {}\n", diff).as_str(),
                );
            } else {
                result.push_str(
                    format!("Minimum functions call gas cost increased by {}\n", -diff).as_str(),
                );
            }
        }

        if self.avg_cost != gas_table.avg_cost() {
            let diff = self.avg_cost as isize - gas_table.avg_cost() as isize;
            if diff.is_positive() {
                result.push_str(
                    format!("Average functions call gas cost reduced by {}\n", diff).as_str(),
                );
            } else {
                result.push_str(
                    format!("Average functions call gas cost increased by {}\n", -diff).as_str(),
                );
            }
        }

        if self.max_cost != gas_table.max_cost() {
            let diff = self.max_cost as isize - gas_table.max_cost() as isize;
            if diff.is_positive() {
                result.push_str(
                    format!("Maximum functions call gas cost reduced by {}\n", diff).as_str(),
                );
            } else {
                result.push_str(
                    format!("Maximum functions call gas cost increased by {}\n", -diff).as_str(),
                );
            }
        }

        result
    }
}

#[derive(Debug)]
pub struct GasTable<'g> {
    contracts: Vec<Contract<'g>>,
}

#[derive(Debug)]
pub struct Contract<'c> {
    file: &'c str,
    contract: &'c str,
    c_type: &'c str,
    deployment_cost: usize,
    deployment_size: usize,
    functions: Vec<Function<'c>>,
}

#[derive(Debug)]
pub struct Function<'f> {
    name: &'f str,
    min: usize,
    avg: usize,
    median: usize,
    max: usize,
    calls: usize,
}

impl GasTable<'_> {
    pub fn deployment_cost(&self) -> usize {
        self.contracts
            .iter()
            .fold(0, |acc, contract| acc + contract.deployment_cost)
    }

    pub fn min_cost(&self) -> usize {
        self.contracts.iter().fold(0, |acc, contract| {
            acc + contract
                .functions
                .iter()
                .fold(0, |acc, function| acc + function.min)
        })
    }

    pub fn avg_cost(&self) -> usize {
        self.contracts.iter().fold(0, |acc, contract| {
            acc + contract
                .functions
                .iter()
                .fold(0, |acc, function| acc + function.avg)
        })
    }

    pub fn median_cost(&self) -> usize {
        self.contracts.iter().fold(0, |acc, contract| {
            acc + contract
                .functions
                .iter()
                .fold(0, |acc, function| acc + function.median)
        })
    }

    pub fn max_cost(&self) -> usize {
        self.contracts.iter().fold(0, |acc, contract| {
            acc + contract
                .functions
                .iter()
                .fold(0, |acc, function| acc + function.max)
        })
    }
}

impl From<&GasTable<'_>> for Snapshot {
    fn from(value: &GasTable) -> Self {
        Snapshot {
            deployment_cost: value.deployment_cost(),
            min_cost: value.min_cost(),
            avg_cost: value.avg_cost(),
            max_cost: value.max_cost(),
        }
    }
}
