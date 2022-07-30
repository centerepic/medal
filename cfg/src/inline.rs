use graph::NodeId;
use ast::{SideEffects, Traverse};
use crate::{ssa_def_use::{SsaDefUse, Location}, function::Function};

pub fn inline_expressions(function: &mut Function, node: NodeId, def_use: &SsaDefUse) {
    let block = function.block(node).unwrap();
    let mut replacements = Vec::new();
    for index in (0..block.len()).rev() {
        if let ast::Statement::Assign(assign) = &block[index] {
            if assign.left.len() != 1 || assign.right.len() != 1 {
                continue;
            }
            if let ast::LValue::Local(target) = &assign.left[0] {
                if let Some(references) = def_use.references.get(&target.0.to_string()) {
                    if references.len() != 1 {
                        continue;
                    }
                    if let Location::Block(ref_node, ref_index) = references[0] {
                        if ref_node != node || ref_index <= index {
                            continue;
                        }
                        if block.iter().skip(index + 1).take(ref_index - index - 1).any(|statement| statement.has_side_effects()) {
                            continue;
                        }
                        replacements.push((index, ref_index, target.clone(), assign.right[0].clone()));
                    }
                }
            }
        }
    }
    let block = function.block_mut(node).unwrap();
    for (_, ref_index, local, new_expression) in &replacements {
        let ref_statement = block.get_mut(*ref_index).unwrap();
        ref_statement.traverse_rvalues(&|rvalue| {
            if let ast::RValue::Local(rvalue_local) = rvalue {
                if rvalue_local == local {
                    *rvalue = new_expression.clone();
                }
            }
        });
    }

    for (index, _, _, _) in replacements {
        block.remove(index);
    }
}