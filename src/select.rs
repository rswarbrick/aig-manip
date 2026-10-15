// Copyright lowRISC contributors.
// Licensed under the Apache License, Version 2.0, see LICENSE for details.
// SPDX-License-Identifier: Apache-2.0

// Select properties of interest in the aig file
use crate::ywmap::YWMap;
use std::io::BufReader;

pub fn select(
    mut aig: aig::Aig,
    out: String,
    yw: String,
    step: usize,
    prop: Vec<String>,
    no_assumes: bool,
) -> Result<(), String> {
    let ywmap = YWMap::parse(BufReader::new(
        std::fs::File::open(&yw).expect("Could not open YW file"),
    ));

    assert_eq!(aig.justice.len(), 0);
    assert_eq!(aig.fairness.len(), 0);

    assert_eq!(aig.bads.len(), ywmap.asserts.len());

    let mut bn = 0; // Index in list of names
    let mut b = 0; // Index in what remains of the list of bads in the AIG
    while b < aig.bads.len() {
        let name = &ywmap.asserts[bn][0][6..]
            .split_once('$')
            .expect("Expected \\top.*$n")
            .0;
        let (stepn, name) = name.split_once('_').expect("badly formed property");
        if !stepn.starts_with("Step") {
            return Err(format!("Property name {stepn} doesn't start with 'Step'"));
        }

        let pstep: usize = stepn[4..].parse().expect("badly formed property name");
        let is_cover = name.ends_with("_Cover");
        bn += 1;

        if pstep == step && (prop.is_empty() || prop.iter().any(|x| x == name)) {
            if is_cover {
                // We encode covers as asserts since otherwise they are lost before going into the AIG file,
                // this means we need to negate them if we want to do BMC for them.
                println!("Keeping {name} (cover statement, negating)");
                aig.bads[b] = !aig.bads[b];
            } else {
                println!("Keeping {name}");
            }
            b += 1;
        } else if no_assumes || is_cover || pstep >= step {
            println!("Deleting {}", name);
            aig.bads.remove(b);
        } else if pstep < step {
            println!("Assert2Assume {}", name);
            aig.constraints.push(!aig.bads[b]);
            aig.bads.remove(b);
        } else {
            unreachable!()
        }
    }

    aig.to_file(&out, false);
    Ok(())
}
