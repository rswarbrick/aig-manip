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

/// What to do with a single assert from the AIG.
#[derive(Debug, PartialEq, Eq)]
enum Action {
    /// Keep the property as something to be proven
    Keep,
    /// Keep a cover property, negating it so BMC looks for a trace that hits it
    KeepNegated,
    /// Remove the property entirely
    Delete,
    /// Turn the assert into an assumption (a constraint)
    ToAssume,
}

/// Decide what to do with a property (keep, negate, delete or assume)
fn classify(prop: &PropName, step: usize, wanted: &[String], no_assumes: bool) -> Action {
    if prop.step > step {
        // The property is for a later step. Discard it.
        Action::Delete
    } else if prop.step == step {
        // The property is for this step. If there is a nonempty list of wanted properties, that
        // might filter out this one. If not, we want to keep it.
        //
        // Keep the property in negated form if it's a cover property. That will mean that the tool
        // is "trying to prove the negation of the property". That would mean finding an example of
        // the double negation, which is what we want.
        if wanted.is_empty() || wanted.iter().any(|x| x == prop.name) {
            if prop.is_cover {
                Action::KeepNegated
            } else {
                Action::Keep
            }
        } else {
            Action::Delete
        }
    } else {
        // The property is for an earlier step. If it was a cover property, that won't be useful for
        // this step (we don't carry existentials as assumptions!). If not, it can be kept as an
        // assumption (which can be disabled by passing no_assumes = true).
        if no_assumes || prop.is_cover {
            Action::Delete
        } else {
            Action::ToAssume
        }
    }
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
        let parsed = parse_prop_name(&ywmap.asserts[bn][0])?;
        let name = parsed.name;
        bn += 1;

        match classify(&parsed, step, &prop, no_assumes) {
            Action::Keep => {
                println!("Keeping {name}");
                b += 1;
            }
            Action::KeepNegated => {
                // We encode covers as asserts since otherwise they are lost before going into the AIG file,
                // this means we need to negate them if we want to do BMC for them.
                println!("Keeping {name} (cover statement, negating)");
                aig.bads[b] = !aig.bads[b];
                b += 1;
            }
            Action::Delete => {
                println!("Deleting {name}");
                aig.bads.remove(b);
            }
            Action::ToAssume => {
                println!("Assert2Assume {name}");
                aig.constraints.push(!aig.bads[b]);
                aig.bads.remove(b);
            }
        }
    }

    aig.to_file(&out, false);
    Ok(())
}
