// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Jayashankar R

//! Simple CLI for testing the normalizer.

use mlnorm::normalize;
use std::io::{self, Read};

fn main() -> io::Result<()> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let output = normalize(&input);
    print!("{}", output);
    Ok(())
}
