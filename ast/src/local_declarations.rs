use std::{collections::{BTreeSet, BTreeMap}, ops::Sub};

use indexmap::IndexSet;
use itertools::Itertools;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{Block, LocalRw, RcLocal, Statement, Assign, Literal, NumericFor};

fn collect_block_locals(block: &Block, locals: &mut IndexSet<RcLocal>) {
    for stat in &block.0 {
        collect_stat_locals(stat, locals);
    }
}

fn collect_stat_locals(stat: &Statement, locals: &mut IndexSet<RcLocal>) {
        locals.extend(stat.values().into_iter().cloned());
        // TODO: traverse_values
        match stat {
            Statement::If(r#if) => {
                collect_block_locals(&r#if.then_block, locals);
                collect_block_locals(&r#if.else_block, locals);
            }
            Statement::While(r#while) => {
                collect_block_locals(&r#while.block, locals);
            }
            Statement::Repeat(repeat) => {
                collect_block_locals(&repeat.block, locals);
            }
            Statement::NumericFor(numeric_for) => {
                collect_block_locals(&numeric_for.block, locals);
            }
            Statement::GenericFor(generic_for) => {
                collect_block_locals(&generic_for.block, locals);
            }
            _ => {}
        }
}

pub fn declare_local(block: &mut Block, local: &RcLocal) {
    let mut usages = BTreeSet::new();
    for (stat_index, stat) in block.iter().enumerate() {
        let mut locals = IndexSet::new();
        collect_stat_locals(stat, &mut locals);
        for used_local in locals {
            if used_local == *local {
                usages.insert(stat_index);
            }
        }
    }

    let mut usages = usages.into_iter();
    let first_stat_index = usages.next().unwrap();
    let declared = if usages.next().is_none() {
        // single usage in this block, declare the local inside the statement
        // if possible
        match &mut block[first_stat_index] {
            Statement::If(r#if) if !r#if.values().into_iter().contains(local) => {
                let mut then_locals = IndexSet::new();
                let mut else_locals = IndexSet::new();
                collect_block_locals(&r#if.then_block, &mut then_locals);
                collect_block_locals(&r#if.else_block, &mut else_locals);
                let then_contains_local = then_locals.into_iter().contains(local);
                let else_contains_local = else_locals.into_iter().contains(local);
                if then_contains_local && !else_contains_local {
                    declare_local(&mut r#if.then_block, local);
                    true
                } else if else_contains_local && !then_contains_local {
                    declare_local(&mut r#if.else_block, local);
                    true
                } else {
                    false
                }
            },
            Statement::While(r#while) if !r#while.values().into_iter().contains(local) => {
                declare_local(&mut r#while.block, local);
                true
            },
            Statement::Repeat(repeat) => {
                declare_local(&mut repeat.block, local);
                true
            },
            Statement::NumericFor(numeric_for) if numeric_for.values_written().into_iter().contains(local) => {
                true
            },
            Statement::GenericFor(generic_for) if generic_for.values_written().into_iter().contains(local) => {
                true
            },
            Statement::NumericFor(numeric_for) if !numeric_for.values().into_iter().contains(local) => {
                declare_local(&mut numeric_for.block, local);
                true
            },
            Statement::GenericFor(generic_for) if !generic_for.values().into_iter().contains(local) => {
                declare_local(&mut generic_for.block, local);
                true
            },
            _ => false,
        }
    } else {
        false
    };

    if !declared {
        // we still need to declare the local
        match &mut block[first_stat_index] {
            stat @ Statement::NumericFor(_) | stat @ Statement::GenericFor(_) 
                if stat.values_written().into_iter().contains(local) => unreachable!(),
            Statement::Assign(assign) if assign.left.iter().exactly_one().ok().and_then(|l| l.as_local()) == Some(local) => {
                assign.prefix = true;
            },
            _ => {
                if first_stat_index > 0 && let Statement::Assign(assign) = &mut block[first_stat_index - 1] && assign.prefix && assign.right.is_empty() {
                    assign.left.push(local.clone().into());
                } else {
                    let mut declaration = Assign::new(vec![local.clone().into()], vec![]);
                    declaration.prefix = true;
                    block.insert(first_stat_index, declaration.into()); 
                }
            }
        }
    }
}

pub fn declare_locals(
    block: &mut Block,
    locals_to_ignore: &FxHashSet<RcLocal>,
) {
    let mut locals = IndexSet::new();
    collect_block_locals(block, &mut locals);
    locals.retain(|l| !locals_to_ignore.contains(l));
    for local in locals {
        declare_local(block, &local);
    }
}
