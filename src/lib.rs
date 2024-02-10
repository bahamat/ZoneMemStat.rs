/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

/*
 * Copyright 2024 MNX Cloud, Inc.
 */

#![crate_name = "zonememstat"]

use serde::Serialize;

use tokio_process_stream::ProcessLineStream;
use tokio::process::Command;
use tokio_stream::StreamExt;

/// The global zone swap usage is not calculated by zonememstat, but it still
/// may be useful to be able to get allocated RSS and max memory for the global
/// zone rather than omitting the entire record.
#[derive(Debug, PartialEq, Serialize)]
pub enum Swap {
    Float(f64),
    None,
}

/// Not all zones have an alias. zonememstat will print these as `-`. For zones
/// with no alias, we will represent that as None.
#[derive(Debug, PartialEq, Serialize)]
pub enum Alias {
    String(String),
    None,
}

/// A struct representing the memory statistics of a single zone (including the
/// global zone) as represented by the `zonememstat` command..
/// See [`zonememstat(8)`](https://smartos.org/man/8/zonememstat) for additional
/// information.
/// For the sake of consistency, the struct member names are the same as the
/// output columns from the `zonememstat`.
#[derive(Debug, PartialEq, Serialize)]
pub struct ZoneMemStat {
    /// The zone name. This will be a uuid.
    pub zonename: String,
    /// The zone alias. Not all zones are assigned an alias.
    pub alias: Alias,
    /// Total size of objects in memory accounted for the zone.
    pub rss: u64,
    /// The zone memory cap in MB. `0` means unlimited.
    pub cap: u64,
    /// The number of times the zone has been over its cap.
    pub nover: u32,
    /// The total amount of memory in MB paged out when the zone has gone over
    /// its cap.
    pub pout: u64,
    /// The percent of swap used against the swap cap.
    pub swap: Swap,
}

/// Takes no input. Returns the output from `zonememstat -Ha` as async.
async fn get_state() -> Result<Vec<ZoneMemStat>, Box<dyn std::error::Error>> {
    let zms = "zonememstat";
    let args = ["-H", "-a"];

    let mut result: Vec<ZoneMemStat> = Vec::new();

    let mut procstream: ProcessLineStream = Command::new(zms)
        .args(&args)
        .try_into()?;

    while let Some(line) = procstream.next().await {
        match line.stdout() {
            Some(l) => result.push(parse_line(l)),
            None => ()
        };
    }

    Ok(result)
}

/// Parse a single line from `zonememstat`
fn parse_line(x: &str) -> ZoneMemStat {
    let split = x.split_whitespace();
    let splitvec: Vec<&str> = split.collect();

    let swap = match splitvec[6].parse::<f64>() {
        Ok(f) => Swap::Float(f),
        Err(_) => Swap::None,
    };

    let alias = match splitvec[1] {
        "-" => Alias::None,
        _ => Alias::String(splitvec[1].to_string()),
    };

    ZoneMemStat {
        zonename: splitvec[0].to_string(),
        alias,
        rss: splitvec[2].parse().expect("Expected a string"),
        cap: splitvec[3].parse().expect("Expected a string"),
        nover: splitvec[4].parse().expect("Expected a string"),
        pout: splitvec[5].parse().expect("Expected a string"),
        swap,
    }
}

/// Takes no input. Returns a Vec of ZoneMemStat structs. The global zone will
/// always be element `0`.
pub async fn stat() -> Vec<ZoneMemStat> {
    match get_state().await {
        Ok(v) => v,
        Err(err) => {
            eprintln!("Error executing zonememstat: {:?}", err);
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gz() {
        let parsed = parse_line("                               global            -      850 16777215        0         0     -");

        // Define the expected values
        let expected = ZoneMemStat {
            zonename: "global".to_string(),
            alias: Alias::None,
            rss: 850,
            cap: 16777215,
            nover: 0,
            pout: 0,
            swap: Swap::None,
        };

        // Compare the actual instance with the expected instance
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_parse_ngz() {
        let parsed = parse_line(" 6dc5da73-e4e5-45b6-80b9-5d2073e9b1ee        amon0      174   1024        0         0 7.11193");

        // Define the expected values
        let expected = ZoneMemStat {
            zonename: "6dc5da73-e4e5-45b6-80b9-5d2073e9b1ee".to_string(),
            alias: Alias::String("amon0".to_string()),
            rss: 174,
            cap: 1024,
            nover: 0,
            pout: 0,
            swap: Swap::Float("7.11193".parse().expect("wut?")),
        };

        // Compare the actual instance with the expected instance
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_parse_ngz_no_alias() {
        let parsed = parse_line(" 6dc5da73-e4e5-45b6-80b9-5d2073e9b1ee            -      174   1024        0         0 7.11193");

        // Define the expected values
        let expected = ZoneMemStat {
            zonename: "6dc5da73-e4e5-45b6-80b9-5d2073e9b1ee".to_string(),
            alias: Alias::None,
            rss: 174,
            cap: 1024,
            nover: 0,
            pout: 0,
            swap: Swap::Float("7.11193".parse().expect("wut?")),
        };

        // Compare the actual instance with the expected instance
        assert_eq!(parsed, expected);
    }
}
