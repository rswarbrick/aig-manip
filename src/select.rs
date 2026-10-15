// Copyright lowRISC contributors.
// Licensed under the Apache License, Version 2.0, see LICENSE for details.
// SPDX-License-Identifier: Apache-2.0

// Select properties of interest in the aig file
use crate::ywmap::YWMap;
use std::io::BufReader;

/// A property name from a .ywmap file, split into its components.
struct PropName<'a> {
    /// The BMC step that the property belongs to
    step: usize,
    /// The name of the property, without the "Step<n>_" prefix
    name: &'a str,
    /// True if the property is a cover statement (encoded as an assert)
    is_cover: bool,
}

/// Parse a raw assert name from a .ywmap file.
///
/// Names have the form "$\top.Step<n>_<name>$<id>".
fn parse_prop_name(raw: &str) -> Result<PropName<'_>, String> {
    let rest = raw
        .strip_prefix("$\\top.")
        .ok_or_else(|| format!("Property name {raw} doesn't start with '$\\top.'"))?;
    let (rest, _) = rest
        .split_once('$')
        .ok_or_else(|| format!("Property name {raw} has no '$' after '$\\top.'"))?;
    let (stepn, name) = rest
        .split_once('_')
        .ok_or_else(|| format!("Property name {raw} has no '_' after the step"))?;
    let step = stepn
        .strip_prefix("Step")
        .ok_or_else(|| format!("Property name {stepn} doesn't start with 'Step'"))?
        .parse()
        .map_err(|_| format!("Property name {raw} has a badly formed step number"))?;

    Ok(PropName {
        step,
        name,
        is_cover: name.ends_with("_Cover"),
    })
}

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
        let PropName {
            step: pstep,
            name,
            is_cover,
        } = parse_prop_name(&ywmap.asserts[bn][0])?;
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
