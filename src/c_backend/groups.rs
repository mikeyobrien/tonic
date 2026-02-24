use crate::mir::MirProgram;
use std::collections::BTreeMap;

#[derive(Debug)]
pub(super) struct FunctionGroup {
    pub(super) name: String,
    pub(super) arity: usize,
    pub(super) clause_indices: Vec<usize>,
}

pub(super) fn group_functions(mir: &MirProgram) -> Vec<FunctionGroup> {
    let mut groups = Vec::<FunctionGroup>::new();
    let mut positions = BTreeMap::<(String, usize), usize>::new();

    for (index, function) in mir.functions.iter().enumerate() {
        let key = (function.name.clone(), function.params.len());
        if let Some(position) = positions.get(&key) {
            groups[*position].clause_indices.push(index);
            continue;
        }
        positions.insert(key, groups.len());
        groups.push(FunctionGroup {
            name: function.name.clone(),
            arity: function.params.len(),
            clause_indices: vec![index],
        });
    }
    groups
}

pub(super) fn group_requires_dispatcher(group: &FunctionGroup, mir: &MirProgram) -> bool {
    if group.clause_indices.len() > 1 {
        return true;
    }
    let function = &mir.functions[group.clause_indices[0]];
    function.param_patterns.is_some() || function.guard_ops.is_some()
}
