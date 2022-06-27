use fxhash::{FxHashMap, FxHashSet};
use graph::{algorithms::dfs_tree, NodeId};

use crate::{
    block::BasicBlock,
    function::Function,
    instruction::location::{InstructionIndex, InstructionLocation},
    value::ValueId,
};

#[derive(Debug, Clone)]
pub struct ValueDefUse {
    pub reads: FxHashSet<InstructionLocation>,
    pub writes: FxHashSet<InstructionLocation>,
}

impl ValueDefUse {
    fn new() -> Self {
        Self {
            reads: FxHashSet::default(),
            writes: FxHashSet::default(),
        }
    }

    fn is_empty(&self) -> bool {
        self.reads.is_empty() && self.writes.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct DefUse(FxHashMap<ValueId, ValueDefUse>);

impl DefUse {
    pub fn new(function: &Function) -> Self {
        let mut def_use = Self(FxHashMap::with_capacity_and_hasher(
            function.value_allocator.borrow().next_value_index,
            Default::default(),
        ));
        let dfs = dfs_tree(function.graph(), function.entry().unwrap()).unwrap();
        for &node in dfs.nodes() {
            def_use.update_block(function.block(node).unwrap(), node);
        }
        def_use
    }

    pub fn update_block_phi(&mut self, block: &BasicBlock, node: NodeId) {
        for value_def_use in &mut self.0.values_mut() {
            value_def_use.reads.retain(|location| {
                !(matches!(location.index, InstructionIndex::Phi(_)) && location.node == node)
            });
            value_def_use.writes.retain(|location| {
                !(matches!(location.index, InstructionIndex::Phi(_)) && location.node == node)
            });
        }
        for index in block
            .phi_instructions
            .iter()
            .enumerate()
            .map(|(i, _)| InstructionIndex::Phi(i))
        {
            for value_read in block.values_read(index) {
                self.0
                    .entry(value_read)
                    .or_insert_with(ValueDefUse::new)
                    .reads
                    .insert(InstructionLocation { node, index });
            }
            for value_written in block.values_written(index) {
                self.0
                    .entry(value_written)
                    .or_insert_with(ValueDefUse::new)
                    .writes
                    .insert(InstructionLocation { node, index });
            }
        }
    }

    pub fn update_block(&mut self, block: &BasicBlock, node: NodeId) {
        for value_def_use in &mut self.0.values_mut() {
            value_def_use.reads.retain(|location| location.node != node);
            value_def_use
                .writes
                .retain(|location| location.node != node);
        }
        for &index in block.indices().iter() {
            for value_read in block.values_read(index) {
                self.0
                    .entry(value_read)
                    .or_insert_with(ValueDefUse::new)
                    .reads
                    .insert(InstructionLocation { node, index });
            }
            for value_written in block.values_written(index) {
                self.0
                    .entry(value_written)
                    .or_insert_with(ValueDefUse::new)
                    .writes
                    .insert(InstructionLocation { node, index });
            }
        }
    }

    pub fn remove_unused(&mut self) {
        self.0.retain(|_, value_def_use| !value_def_use.is_empty());
    }

    pub fn values(&self) -> impl Iterator<Item = ValueId> + '_ {
        self.0.keys().cloned()
    }

    pub fn get(&self, value: ValueId) -> Option<&ValueDefUse> {
        self.0.get(&value)
    }
}